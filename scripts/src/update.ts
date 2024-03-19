import {
    Connection,
    Keypair, PublicKey,
    TransactionInstruction, TransactionMessage, VersionedTransaction
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
    let tokenAccountKeypair = new Keypair();
    let adminKeyPair: Keypair;
    try {
        adminKeyPair = getKeypair("admin", network);
    } catch (e) {
        console.log("Error getting admin keypair: ", e);
        return
    }

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
    const latestBlockHash = await connection.getLatestBlockhash('confirmed');
    const messageV0 = new TransactionMessage({
        payerKey: adminKeyPair.publicKey,
        recentBlockhash: latestBlockHash.blockhash,
        instructions: [updateAPYIX]
    }).compileToV0Message();
    const transaction = new VersionedTransaction(messageV0);
    transaction.sign([adminKeyPair, tokenAccountKeypair]);
    const txId = await connection.sendTransaction(transaction, {maxRetries: 5});
    const confirmation = await connection.confirmTransaction({
        signature: txId,
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight
    });
    if (confirmation.value.err) {
        console.log(`Transaction could not be confirmed ❌❌❌`);
    } else {
        console.log(`Transaction confirmed successfully ✅✅✅`)
    }
}

updateAPY(
    "mainnet",
    10,
    7*24*60*60
).then((val) => console.log(val))