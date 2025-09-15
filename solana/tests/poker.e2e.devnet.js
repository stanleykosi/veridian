const fs = require('fs');
const os = require('os');
const anchor = require('@coral-xyz/anchor');
const { PublicKey, Keypair, Connection, LAMPORTS_PER_SOL, SystemProgram } = require('@solana/web3.js');
const { expect } = require('chai');
const {
  awaitComputationFinalization,
  getCompDefAccOffset,
  getArciumAccountBaseSeed,
  getArciumProgAddress,
  getMXEAccAddress,
  getMempoolAccAddress,
  getExecutingPoolAccAddress,
  getComputationAccAddress,
  getCompDefAccAddress,
  ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
  getClockAccAddress,
  getClusterAccAddress,
  getStakingPoolAccAddress,
} = require('@arcium-hq/client');
const {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  createMint,
  mintTo,
} = require('@solana/spl-token');

// Helpers
function readKpJson(path) {
  const secret = JSON.parse(fs.readFileSync(path, 'utf-8'));
  return Keypair.fromSecretKey(new Uint8Array(secret));
}

function pda(seedUtf8, seeds, programId) {
  return PublicKey.findProgramAddressSync([
    Buffer.from(seedUtf8, 'utf8'),
    ...seeds,
  ], programId)[0];
}

function u64le(n) {
  return Buffer.from(new Uint8Array(new BigUint64Array([BigInt(n)]).buffer));
}

async function ensureAirdrop(conn, pubkey, minSol = 0.5) {
  const bal = await conn.getBalance(pubkey);
  if (bal < minSol * LAMPORTS_PER_SOL) {
    await conn.requestAirdrop(pubkey, Math.ceil((minSol * LAMPORTS_PER_SOL) - bal));
    await new Promise(r => setTimeout(r, 2000));
  }
}

describe('Poker E2E (devnet)', () => {
  // Provider on devnet using default solana keypair
  const wallet = new anchor.Wallet(readKpJson(`${os.homedir()}/.config/solana/id.json`));
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  const provider = new anchor.AnchorProvider(connection, wallet, { commitment: 'confirmed' });
  anchor.setProvider(provider);

  // Load program via workspace artifacts
  const program = anchor.workspace.VeridianHoldem;
  const programId = program.programId;

  // Test constants
  const tableId = 1n;
  const smallBlind = 1000n;
  const bigBlind = 2000n;
  const buyIn = 100_000n; // 100k units of mint (9 decimals default)

  it('runs encrypted deal + reveals (devnet)', async () => {
    // Derive PDAs for table
    const tableConfigPda = pda('table_config', [u64le(tableId)], programId);
    const gamePda = pda('game', [u64le(tableId)], programId);
    const escrowPda = pda('escrow', [gamePda.toBuffer()], programId);
    const handPda = pda('hand', [gamePda.toBuffer()], programId);
    const signPda = pda('sign_pda', [], programId);

    // Create SPL mint for the table currency (creator is mint authority)
    await ensureAirdrop(connection, wallet.publicKey, 2);
    const mint = await createMint(
      connection,
      wallet.payer,
      wallet.publicKey,
      null,
      9 // decimals
    );
    // Creator ATA and funding (use helper for reliability)
    const creatorAta = (await getOrCreateAssociatedTokenAccount(connection, wallet.payer, mint, wallet.publicKey)).address;
    await mintTo(connection, wallet.payer, mint, creatorAta, wallet.payer, Number(buyIn));

    // Try create_table if accounts not present yet
    let tableExists = false;
    try {
      await program.account.gameState.fetch(gamePda);
      tableExists = true;
    } catch { }

    if (!tableExists) {
      await program.methods
        .createTable(
          new anchor.BN(tableId.toString()),
          new anchor.BN(smallBlind.toString()),
          new anchor.BN(bigBlind.toString()),
          new anchor.BN(buyIn.toString()),
        )
        .accounts({
          tableConfig: tableConfigPda,
          gameState: gamePda,
          escrowAccount: escrowPda,
          creator: wallet.publicKey,
          tokenMint: mint,
          creatorTokenAccount: creatorAta,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .rpc();
    }

    // Join second player if needed
    const gameNow = await program.account.gameState.fetch(gamePda);
    if (gameNow.players[1].equals(PublicKey.default)) {
      const joiner = Keypair.generate();
      await ensureAirdrop(connection, joiner.publicKey, 1);
      // Create joiner ATA and fund buy-in using helper
      const joinerAta = (await getOrCreateAssociatedTokenAccount(connection, wallet.payer, mint, joiner.publicKey)).address;
      await mintTo(connection, wallet.payer, mint, joinerAta, wallet.payer, Number(buyIn));

      await program.methods
        .joinTable()
        .accounts({
          gameState: gamePda,
          tableConfig: tableConfigPda,
          escrowAccount: escrowPda,
          joiner: joiner.publicKey,
          joinerTokenAccount: joinerAta,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([joiner])
        .rpc();
    }

    // Arcium accounts
    const arciumProgram = getArciumProgAddress();
    const mxeAccount = getMXEAccAddress(programId);
    const mempoolAccount = getMempoolAccAddress(programId);
    const executingPool = getExecutingPoolAccAddress(programId);
    const clusterAccount = getClusterAccAddress(mxeAccount);
    const poolAccount = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS ?? getStakingPoolAccAddress();
    const clockAccount = getClockAccAddress();
    const instructionsSysvar = anchor.web3.SYSVAR_INSTRUCTIONS_PUBKEY;
    const systemProgram = SystemProgram.programId;

    // Sanity: ensure Arcium PDAs exist
    async function mustExist(pubkeyLike, name) {
      expect(pubkeyLike, `${name} undefined`).to.exist;
      const key = pubkeyLike instanceof PublicKey ? pubkeyLike : new PublicKey(pubkeyLike);
      const info = await connection.getAccountInfo(key);
      expect(info, `${name} missing on devnet: ${key.toBase58()}`).to.exist;
    }
    await mustExist(mxeAccount, 'mxeAccount');
    await mustExist(mempoolAccount, 'mempoolAccount');
    await mustExist(executingPool, 'executingPool');
    await mustExist(clusterAccount, 'clusterAccount');
    await mustExist(poolAccount, 'poolAccount');
    await mustExist(clockAccount, 'clockAccount');

    // -------- Deal New Hand --------
    const dealOffsetBN = new anchor.BN(700);
    const compAccDeal = getComputationAccAddress(programId, dealOffsetBN);
    // Resolve comp def PDA with endian fallback
    const hashBytes = getCompDefAccOffset('shuffle_and_deal');
    let compDefAccountDeal = getCompDefAccAddress(programId, Buffer.from(hashBytes).readUInt32LE(0));
    let infoDeal = await connection.getAccountInfo(compDefAccountDeal);
    if (!infoDeal) {
      const be = Buffer.from(hashBytes).readUInt32BE(0);
      compDefAccountDeal = getCompDefAccAddress(programId, be);
      infoDeal = await connection.getAccountInfo(compDefAccountDeal);
    }
    expect(infoDeal, 'compDefAccount for shuffle_and_deal not found').to.exist;

    await program.methods
      .dealNewHand(dealOffsetBN)
      .accounts({
        payer: wallet.publicKey,
        gameState: gamePda,
        handState: handPda,
        signPdaAccount: signPda,
        mxeAccount,
        mempoolAccount,
        executingPool,
        computationAccount: compAccDeal,
        compDefAccount: compDefAccountDeal,
        clusterAccount,
        poolAccount,
        clockAccount,
        instructionsSysvar,
        systemProgram,
        arciumProgram,
      })
      .rpc();

    await awaitComputationFinalization(provider, compAccDeal, programId, 'confirmed');

    const afterDeal = await program.account.gameState.fetch(gamePda);
    expect(afterDeal.gamePhase).to.equal(2); // PreFlop

    // -------- Reveal Community Cards (Flop, Turn, River) --------
    async function reveal(offsetNumber) {
      const offsetBN = new anchor.BN(offsetNumber);
      const compAcc = getComputationAccAddress(programId, offsetBN);
      const hash = getCompDefAccOffset('reveal_community_cards');
      let compDefAccount = getCompDefAccAddress(programId, Buffer.from(hash).readUInt32LE(0));
      let info = await connection.getAccountInfo(compDefAccount);
      if (!info) {
        compDefAccount = getCompDefAccAddress(programId, Buffer.from(hash).readUInt32BE(0));
        info = await connection.getAccountInfo(compDefAccount);
      }
      expect(info, 'compDefAccount for reveal_community_cards not found').to.exist;

      await program.methods
        .requestCommunityCards(offsetBN)
        .accounts({
          payer: wallet.publicKey,
          gameState: gamePda,
          handState: handPda,
          signPdaAccount: signPda,
          mxeAccount,
          mempoolAccount,
          executingPool,
          computationAccount: compAcc,
          compDefAccount,
          clusterAccount,
          poolAccount,
          clockAccount,
          instructionsSysvar,
          systemProgram,
          arciumProgram,
        })
        .rpc();

      await awaitComputationFinalization(provider, compAcc, programId, 'confirmed');
    }

    await reveal(710); // Flop
    await reveal(711); // Turn
    await reveal(712); // River

    const afterRiver = await program.account.gameState.fetch(gamePda);
    expect(afterRiver.communityCards[0]).to.not.equal(255);
    expect(afterRiver.communityCards[4]).to.not.equal(255);
  });
});
