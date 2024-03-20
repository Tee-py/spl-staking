import {
    Connection,
    Keypair, PublicKey,
    TransactionInstruction, TransactionMessage, VersionedTransaction,
    ComputeBudgetProgram, Transaction
} from "@solana/web3.js";
import BN from "bn.js";
import {getCluster, getKeypair} from "./utils";
import { PROGRAM_ID } from "./constant";

const updateAPY = async (
    network: string,
    normalStakingApy: number,
    lockedStakingApy: number
) => {
    const cluster = getCluster(network);
    const connection = new Connection(cluster, "confirmed");
    const programId = new PublicKey(PROGRAM_ID);
    let adminKeyPair: Keypair;
    try {
        adminKeyPair = getKeypair("admin", network);
    } catch (e) {
        console.log("Error getting admin keypair: ", e);
        return
    }

    //const PRIORITY_RATE = 100; // MICRO_LAMPORTS
    //const PRIORITY_FEE_IX = ComputeBudgetProgram.setComputeUnitPrice({microLamports: PRIORITY_RATE});
    const dataAccountPubKey = new PublicKey("994NbZhmVGDAvXHWW8VMA4kBgeHpKF8xebncag4KnRVE");
    const instructionData = Buffer.from(
        Uint8Array.of(
            3,
            ...new BN(normalStakingApy * 10).toArray("le", 8),
            ...new BN(lockedStakingApy * 10).toArray("le", 8)
        )
    )
    const updateAPYIX = new TransactionInstruction({
        programId,
        keys: [
            { pubkey: adminKeyPair.publicKey, isSigner: true, isWritable: false },
            { pubkey: dataAccountPubKey, isSigner: false, isWritable: true }
        ],
        data: instructionData
    });
    const txn = new Transaction();
    txn.add(updateAPYIX);
    const hash = await connection.sendTransaction(
        txn, [adminKeyPair],
        { skipPreflight: false, preflightCommitment: "confirmed"}
    )
    console.log(hash);
    // const latestBlockHash = await connection.getLatestBlockhash('confirmed');
    // const messageV0 = new TransactionMessage({
    //     payerKey: adminKeyPair.publicKey,
    //     recentBlockhash: latestBlockHash.blockhash,
    //     instructions: [updateAPYIX]
    // }).compileToV0Message();
    // const transaction = new VersionedTransaction(messageV0);
    // transaction.sign([adminKeyPair]);
    // const txId = await connection.sendTransaction(transaction);
    // console.log(`Finalizing Transaction... ${txId}`);
    // const confirmation = await connection.confirmTransaction({
    //     signature: txId,
    //     blockhash: latestBlockHash.blockhash,
    //     lastValidBlockHeight: latestBlockHash.lastValidBlockHeight
    // });
    // if (confirmation.value.err) {
    //     console.log(`Transaction could not be confirmed ❌❌❌`);
    // } else {
    //     console.log(`Transaction confirmed successfully ✅✅✅`);
    //     console.log(`Explorer Link: https://explorer.solana.com/tx/${txId}`);
    // }
    // const maxRetries = 5;
    // let tries = 0;
    // while (tries <= maxRetries) {
    //     try {
    //         const latestBlockHash = await connection.getLatestBlockhash('confirmed');
    //         messageV0.recentBlockhash = latestBlockHash.blockhash;
    //         const transaction = new VersionedTransaction(messageV0);
    //         transaction.sign([adminKeyPair]);
    //         const txId = await connection.sendTransaction(transaction, {maxRetries: 5});
    //         const confirmation = await connection.confirmTransaction({
    //             signature: txId,
    //             blockhash: latestBlockHash.blockhash,
    //             lastValidBlockHeight: latestBlockHash.lastValidBlockHeight
    //         });
    //         if (confirmation.value.err) {
    //             console.log(`Transaction could not be confirmed ❌❌❌`);
    //         } else {
    //             console.log(`Transaction confirmed successfully ✅✅✅`);
    //             console.log(`Explorer Link: https://explorer.solana.com/tx/${txId}`);
    //             break
    //         }
    //     } catch (e) {
    //         console.log(e)
    //         tries++
    //         console.log(`Could not confirm txn... ${maxRetries - tries} remaining`);
    //     }
    // }
}

updateAPY(
    "mainnet",
    1780,
    3127
).then((val) => console.log(val))