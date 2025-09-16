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
const crypto = require('crypto');
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
    const sig = await conn.requestAirdrop(pubkey, Math.ceil((minSol * LAMPORTS_PER_SOL) - bal));
    try {
      const latest = await conn.getLatestBlockhash('processed');
      await conn.confirmTransaction({ signature: sig, ...latest }, 'processed');
    } catch {}
    await new Promise(r => setTimeout(r, 1500));
  }
}

async function resolveFeePoolPda(provider, arciumProgramId) {
  const seedsToTry = [
    "FeePool",
    "FeePoolAccount", 
    "fee_pool",
  ];
  const expectedDisc = anchorDiscriminator("FeePool");
  for (const seed of seedsToTry) {
    const candidate = PublicKey.findProgramAddressSync([Buffer.from(seed)], arciumProgramId)[0];
    const info = await provider.connection.getAccountInfo(candidate);
    if (info && info.data && info.data.length >= 8 && equalBytes(info.data.slice(0, 8), expectedDisc)) {
      return candidate;
    }
  }
  // Fallback to staking pool PDA if FeePool cannot be located
  return getStakingPoolAccAddress();
}

function anchorDiscriminator(name) {
  const preimage = `account:${name}`;
  return crypto.createHash("sha256").update(preimage).digest().slice(0, 8);
}

function equalBytes(a, b) {
  if (!a || !b || a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) { if (a[i] !== b[i]) return false; }
  return true;
}

describe('Poker E2E (devnet)', () => {
  // Provider on devnet using default solana keypair
  const wallet = new anchor.Wallet(readKpJson(`${os.homedir()}/.config/solana/id.json`));
  const connection = new Connection('https://api.devnet.solana.com', { commitment: 'processed' });
  const provider = new anchor.AnchorProvider(connection, wallet, { commitment: 'processed', preflightCommitment: 'processed' });
  anchor.setProvider(provider);

  // Load program via workspace artifacts
  const program = anchor.workspace.VeridianHoldem;
  const programId = program.programId;
  console.log('Using program ID:', programId.toBase58());

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
    // Use the cluster account that was created during deployment
    const clusterAccount = getClusterAccAddress(mxeAccount);
    // Resolve the fee pool account using the proper method
    const poolAccount = await resolveFeePoolPda(provider, arciumProgram);
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

    // -------- Deal New Hand Setup --------
    const dealOffsetBN = new anchor.BN(800); // Use a different offset to avoid conflicts

    // Check if hand_state account already exists
    let handStateExists = false;
    try {
      await program.account.handState.fetch(handPda);
      handStateExists = true;
      console.log('Hand state account already exists, skipping setup');
    } catch (e) {
      console.log('Hand state account does not exist, creating it');
    }

    // Only setup if hand_state doesn't exist
    if (!handStateExists) {
      await program.methods
        .dealNewHandSetup(dealOffsetBN)
        .accounts({
          payer: wallet.publicKey,
          gameState: gamePda,
          handState: handPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }

    // -------- Deal New Hand Queue --------
    const compAccDeal = getComputationAccAddress(programId, dealOffsetBN);
    // Derive CompDef PDA using our programId (MXE program id) per Arcium docs
    const compDefOffsetBytes = Buffer.from(getCompDefAccOffset('shuffle_and_deal'));
    const compDefOffsetU32LE = compDefOffsetBytes.readUInt32LE(0);
    const shuffleAndDealCompDefAccount = getCompDefAccAddress(programId, compDefOffsetU32LE);
    console.log('=== DEBUGGING COMP DEF DERIVATION ===');
    console.log('Program ID:', programId.toBase58());
    console.log('Offset bytes:', compDefOffsetBytes.toString('hex'));
    console.log('Derived comp_def_account (expected):', shuffleAndDealCompDefAccount.toBase58());
    const existsShuffle = await connection.getAccountInfo(shuffleAndDealCompDefAccount);
    console.log('Exists on-chain?', !!existsShuffle);
    console.log('=== END DEBUGGING ===');

    // Debug the accounts object before sending
    const accountsObject = {
      payer: wallet.publicKey,
      gameState: gamePda,
      handState: handPda,
      signPdaAccount: signPda,
      mxeAccount,
      mempoolAccount,
      executingPool,
      computationAccount: compAccDeal,
      compDefAccount: shuffleAndDealCompDefAccount,
      clusterAccount,
      poolAccount,
      clockAccount,
      instructionsSysvar,
      systemProgram,
      arciumProgram,
    };
    
    console.log('=== ACCOUNTS OBJECT DEBUG ===');
    console.log('compDefAccount in accounts object:', accountsObject.compDefAccount.toBase58());
    console.log('shuffleAndDealCompDefAccount variable:', shuffleAndDealCompDefAccount.toBase58());
    console.log('Are they the same?', accountsObject.compDefAccount.equals(shuffleAndDealCompDefAccount));
    console.log('=== END ACCOUNTS OBJECT DEBUG ===');

    // Build instruction to inspect which keys are sent and in what order
    const builtIx = await program.methods
      .dealNewHandQueue(dealOffsetBN)
      .accounts(accountsObject)
      .instruction();
    console.log('=== BUILT INSTRUCTION KEYS (index -> pubkey) ===');
    builtIx.keys.forEach((k, i) => console.log(`${i}: ${k.pubkey.toBase58()}`));
    console.log('=== END BUILT INSTRUCTION KEYS ===');

    await program.methods
      .dealNewHandQueue(dealOffsetBN)
      .accounts(accountsObject)
      .rpc({ commitment: 'processed', skipPreflight: false, maxRetries: 3 });

    await awaitComputationFinalization(provider, compAccDeal, programId, 'confirmed');

    const afterDeal = await program.account.gameState.fetch(gamePda);
    expect(afterDeal.gamePhase).to.equal(2); // PreFlop

    // -------- Reveal Community Cards (Flop, Turn, River) --------
    async function reveal(offsetNumber) {
      const offsetBN = new anchor.BN(offsetNumber);
      const compAcc = getComputationAccAddress(programId, offsetBN);
      // According to Arcium docs: offset should be interpreted as little-endian u32
      const compDefOffsetBytes = Buffer.from(getCompDefAccOffset('reveal_community_cards'));
      const compDefAccountReveal = getCompDefAccAddress(programId, compDefOffsetBytes.readUInt32LE(0));
      const info = await connection.getAccountInfo(compDefAccountReveal);
      expect(info, 'compDefAccount for reveal_community_cards not found (run init_comp_defs.ts)').to.exist;

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
          compDefAccount: compDefAccountReveal,
          clusterAccount,
          poolAccount,
          clockAccount,
          instructionsSysvar,
          systemProgram,
          arciumProgram,
        })
        .rpc({ commitment: 'processed', skipPreflight: false, maxRetries: 3 });

      await awaitComputationFinalization(provider, compAcc, programId, 'confirmed');
    }

    await reveal(710); // Flop
    await reveal(711); // Turn
    await reveal(712); // River

    const afterRiver = await program.account.gameState.fetch(gamePda);
    expect(afterRiver.communityCards[0]).to.not.equal(255);
    expect(afterRiver.communityCards[4]).to.not.equal(255);

    // -------- Request Showdown (Test the stack overflow fix) --------
    const showdownOffsetBN = new anchor.BN(720);
    const compAccShowdown = getComputationAccAddress(programId, showdownOffsetBN);
    // According to Arcium docs: offset should be interpreted as little-endian u32
    const compDefOffsetBytesShowdown = Buffer.from(getCompDefAccOffset('determine_winner'));
    const compDefAccountShowdown = getCompDefAccAddress(programId, compDefOffsetBytesShowdown.readUInt32LE(0));
    const infoShowdown = await connection.getAccountInfo(compDefAccountShowdown);
    expect(infoShowdown, 'compDefAccount for determine_winner not found (run init_comp_defs.ts)').to.exist;

    // Update game state to Showdown phase for the test
    // Note: In a real game, this would happen after betting rounds
    console.log('Testing requestShowdown instruction (stack overflow fix)...');

    await program.methods
      .requestShowdown(showdownOffsetBN)
      .accounts({
        payer: wallet.publicKey,
        gameState: gamePda,
        handState: handPda,
        treasuryTokenAccount: wallet.publicKey, // Using wallet as placeholder
        dealerAccount: wallet.publicKey, // Using wallet as placeholder
        signPdaAccount: signPda,
        mxeAccount,
        mempoolAccount,
        executingPool,
        computationAccount: compAccShowdown,
        compDefAccount: compDefAccountShowdown,
        clusterAccount,
        poolAccount,
        clockAccount,
        instructionsSysvar,
        systemProgram,
        arciumProgram,
      })
      .rpc({ commitment: 'processed', skipPreflight: false, maxRetries: 3 });

    console.log('✅ requestShowdown instruction executed successfully (no stack overflow)!');
  });
});
