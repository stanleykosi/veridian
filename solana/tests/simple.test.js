const fs = require('fs');
const os = require('os');
const anchor = require('@coral-xyz/anchor');
const { PublicKey, Keypair, Connection, LAMPORTS_PER_SOL, SystemProgram } = require('@solana/web3.js');
const { expect } = require('chai');

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

describe('Simple Veridian Holdem Test', () => {
  // Provider on devnet using default solana keypair
  const wallet = new anchor.Wallet(readKpJson(`${os.homedir()}/.config/solana/id.json`));
  const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
  const provider = new anchor.AnchorProvider(connection, wallet, { commitment: 'confirmed' });
  anchor.setProvider(provider);

  // Load program via workspace artifacts
  const program = anchor.workspace.VeridianHoldem;
  const programId = program.programId;

  it('should load the program successfully', async () => {
    console.log('Program ID:', programId.toString());
    expect(programId).to.exist;
    expect(programId.toString()).to.equal('Grax8NuUaPo4bA43PiYkAhdLvU7Vts2o8Gk16TdV6ZEQ');
  });

  it('should have the correct program methods', async () => {
    expect(program.methods.dealNewHandSetup).to.exist;
    expect(program.methods.dealNewHandQueue).to.exist;
    expect(program.methods.requestCommunityCards).to.exist;
    expect(program.methods.requestShowdown).to.exist;
    console.log('✅ All required methods exist');
  });

  it('should be able to create a simple table', async () => {
    const tableId = 999n; // Use a unique table ID
    const smallBlind = 1000n;
    const bigBlind = 2000n;
    const buyIn = 100_000n;

    // Derive PDAs for table
    const tableConfigPda = pda('table_config', [u64le(tableId)], programId);
    const gamePda = pda('game', [u64le(tableId)], programId);
    const escrowPda = pda('escrow', [gamePda.toBuffer()], programId);

    // Ensure we have enough SOL
    await ensureAirdrop(connection, wallet.publicKey, 2);

    // Create SPL mint for the table currency
    const { createMint, getOrCreateAssociatedTokenAccount, mintTo } = require('@solana/spl-token');
    const mint = await createMint(
      connection,
      wallet.payer,
      wallet.publicKey,
      null,
      9 // decimals
    );

    // Creator ATA and funding
    const creatorAta = (await getOrCreateAssociatedTokenAccount(connection, wallet.payer, mint, wallet.publicKey)).address;
    await mintTo(connection, wallet.payer, mint, creatorAta, wallet.payer, Number(buyIn));

    // Check if table already exists
    let tableExists = false;
    try {
      await program.account.gameState.fetch(gamePda);
      tableExists = true;
      console.log('Table already exists, skipping creation');
    } catch { }

    if (!tableExists) {
      const tx = await program.methods
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
          tokenProgram: require('@solana/spl-token').TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .rpc();

      console.log('✅ Table created successfully:', tx);
    }

    // Verify the table was created
    const gameState = await program.account.gameState.fetch(gamePda);
    console.log('Game state:', gameState);
    expect(gameState.tableId.toString()).to.equal(tableId.toString());
    expect(gameState.smallBlind.toString()).to.equal(smallBlind.toString());
    expect(gameState.bigBlind.toString()).to.equal(bigBlind.toString());
    expect(gameState.buyIn.toString()).to.equal(buyIn.toString());

    console.log('✅ Table verification successful');
  });
});
