import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { VeridianHoldem } from "./target/types/veridian_holdem";
import {
  getCompDefAccOffset,
  getArciumAccountBaseSeed,
  getArciumProgAddress,
  uploadCircuit,
  buildFinalizeCompDefTx,
  getMXEAccAddress,
} from "@arcium-hq/client";
import * as fs from "fs";
import * as os from "os";

async function main() {
  // Configure the client to use devnet
  const provider = new anchor.AnchorProvider(
    new anchor.web3.Connection("https://api.devnet.solana.com"),
    new anchor.Wallet(anchor.web3.Keypair.fromSecretKey(
      new Uint8Array(JSON.parse(fs.readFileSync(`${os.homedir()}/.config/solana/id.json`, "utf8")))
    )),
    { commitment: "confirmed" }
  );
  anchor.setProvider(provider);

  const program = anchor.workspace.VeridianHoldem as Program<VeridianHoldem>;

  const owner = anchor.web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(fs.readFileSync(`${os.homedir()}/.config/solana/id.json`, "utf8")))
  );

  // Use the same cluster offset from deployment
  const clusterOffset = 1116522165;
  const clusterAccount = PublicKey.findProgramAddressSync(
    [Buffer.from("cluster"), Buffer.from(clusterOffset.toString())],
    getArciumProgAddress()
  )[0];

  console.log("🚀 Initializing Veridian Hold'em Computation Definitions...");
  console.log("Program ID:", program.programId.toString());
  console.log("Cluster Account:", clusterAccount.toString());

  // Initialize all computation definitions with error handling
  const compDefs = [
    {
      name: "shuffle_and_deal",
      circuitPath: "./build/shuffle_and_deal_testnet.arcis",
      methodName: "initShuffleAndDealCompDef"
    },
    {
      name: "reveal_community_cards",
      circuitPath: "./build/reveal_community_cards_testnet.arcis",
      methodName: "initRevealCommunityCardsCompDef"
    },
    {
      name: "determine_winner",
      circuitPath: "./build/determine_winner_testnet.arcis",
      methodName: "initDetermineWinnerCompDef"
    }
  ];

  for (let i = 0; i < compDefs.length; i++) {
    const compDef = compDefs[i];
    try {
      console.log(`\n${i + 1}️⃣ Initializing ${compDef.name} computation definition...`);
      const sig = await initCompDef(
        program,
        owner,
        compDef.name,
        compDef.circuitPath,
        compDef.methodName,
        provider as anchor.AnchorProvider,
        clusterAccount
      );
      console.log(`✅ ${compDef.name} initialized:`, sig);
    } catch (error) {
      if (error.message && error.message.includes("already in use")) {
        console.log(`⚠️  ${compDef.name} already initialized, skipping...`);
      } else {
        console.log(`❌ Error initializing ${compDef.name}:`, error.message);
        throw error; // Re-throw if it's not an "already in use" error
      }
    }
  }

  console.log("\n🎉 All computation definitions initialized successfully!");
  console.log("\nYour Veridian Hold'em program is now ready for confidential poker games!");
}

async function initCompDef(
  program: Program<VeridianHoldem>,
  owner: anchor.web3.Keypair,
  instructionName: string,
  circuitPath: string,
  methodName: string,
  provider: anchor.AnchorProvider,
  clusterAccount: PublicKey
): Promise<string> {
  const baseSeedCompDefAcc = getArciumAccountBaseSeed("ComputationDefinitionAccount");
  const offset = getCompDefAccOffset(instructionName);

  const compDefPDA = PublicKey.findProgramAddressSync(
    [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
    getArciumProgAddress()
  )[0];

  console.log(`Comp def PDA for ${instructionName}:`, compDefPDA.toString());

  // Read the circuit file
  const rawCircuit = fs.readFileSync(circuitPath);

  // Initialize the computation definition with all required accounts
  const sig = await program.methods[methodName]()
    .accounts({
      compDefAccount: compDefPDA,
      payer: owner.publicKey,
      mxeAccount: getMXEAccAddress(program.programId),
      clusterAccount: clusterAccount,
      systemProgram: anchor.web3.SystemProgram.programId,
      arciumProgram: getArciumProgAddress(),
    })
    .signers([owner])
    .rpc({
      commitment: "confirmed",
    });

  console.log(`Init ${instructionName} computation definition transaction:`, sig);

  // Upload the circuit
  console.log(`Uploading circuit for ${instructionName}...`);
  await uploadCircuit(
    provider as anchor.AnchorProvider,
    instructionName,
    program.programId,
    rawCircuit,
    true
  );

  // Finalize the computation definition
  console.log(`Finalizing ${instructionName} computation definition...`);
  const finalizeTx = await buildFinalizeCompDefTx(
    provider as anchor.AnchorProvider,
    Buffer.from(offset).readUInt32LE(),
    program.programId
  );

  const finalizeSig = await provider.sendAndConfirm(finalizeTx, [owner]);
  console.log(`Finalize ${instructionName} transaction:`, finalizeSig);

  return sig;
}

main().catch(console.error);
