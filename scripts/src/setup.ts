import {
    Connection,
    Keypair, PublicKey, sendAndConfirmTransaction, Signer,
    SystemProgram,
    SYSVAR_RENT_PUBKEY, Transaction,
    TransactionInstruction, TransactionMessage, VersionedTransaction
} from "@solana/web3.js";
import BN from "bn.js";
import {
    AccountLayout,
    getOrCreateAssociatedTokenAccount,
    createInitializeAccountInstruction,
    createMint,
    mintTo,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    ExtensionType,
    getMintLen,
    createInitializeTransferFeeConfigInstruction,
    createInitializeMintInstruction
} from "@solana/spl-token";
import {
    PROGRAM_ID, MAX_FEE, FEE_BASIS_POINTS,
    TOKEN_DECIMALS, DEVNET_CONNECTION_URL,
    MAINNET_CONNECTION_URL, LOCALNET_CONNECTION_URL
} from "./constant";

import {getCluster, getKeypair, getPublicKey, writePublicKey, writeSecretKey} from "./utils";

const setupMint = async (
    name: string,
    network: string,
    connection: Connection,
    clientKeypair: Signer,
    mintTokens: boolean,
    mintToAccount?: string
) => {
    console.log(`Creating Mint ${name}...`);
    let mintKeyPair: Keypair;
    try {
        mintKeyPair = getKeypair("mint", network);
    } catch {
        mintKeyPair = new Keypair();
        const extensions = [
            ExtensionType.TransferFeeConfig
        ];
        const minLen = getMintLen(extensions);
        const requiredLamports = await connection.getMinimumBalanceForRentExemption(minLen);
        const mintTxn = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: clientKeypair.publicKey,
                newAccountPubkey: mintKeyPair.publicKey,
                space: minLen,
                lamports: requiredLamports,
                programId: TOKEN_2022_PROGRAM_ID
            }),
            createInitializeTransferFeeConfigInstruction(
                mintKeyPair.publicKey,
                clientKeypair.publicKey,
                clientKeypair.publicKey,
                FEE_BASIS_POINTS,
                BigInt(MAX_FEE),
                TOKEN_2022_PROGRAM_ID
            ),
            createInitializeMintInstruction(mintKeyPair.publicKey, TOKEN_DECIMALS, clientKeypair.publicKey, null, TOKEN_2022_PROGRAM_ID)
        );
        await sendAndConfirmTransaction(connection, mintTxn, [clientKeypair, mintKeyPair], undefined);
        writePublicKey(mintKeyPair.publicKey, "mint", network);
        writeSecretKey(mintKeyPair.secretKey, "mint", network);
    }
    // For Devnet [ Create Token Accounts and Mint Tokens ]
    if (mintTokens && mintToAccount) {
        console.log('Minting Tokens')
        const associatedToken = await getOrCreateAssociatedTokenAccount(
            connection,
            clientKeypair,
            mintKeyPair.publicKey,
            new PublicKey(mintToAccount),
            undefined,
            undefined,
            undefined,
            TOKEN_2022_PROGRAM_ID
        )
        await mintTo(
            connection,
            clientKeypair,
            mintKeyPair.publicKey,
            associatedToken.address,
            clientKeypair,
            20000*10**TOKEN_DECIMALS,
            undefined,
            undefined,
            TOKEN_2022_PROGRAM_ID
        )
        console.log('Done minting for:', associatedToken.address)
    }
};


const setup = async (
    network: string,
    minimumStakeAmount: number,
    minimumLockDuration: number,
    normalStakingApy: number,
    lockedStakingApy: number,
    earlyWithdrawalFee: number,
    runInit: boolean = true
) => {
    let cluster = getCluster(network);
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
    if (network == "devnet" || network == "localnet") {
        await setupMint(
            "Libra",
            network,
            connection,
            adminKeyPair,
            true,
            "FVHN3NdiUvfdzWRGji9uFzGALqSy7u2qF2zcwZcRTgmV"
        )
    }
    if (runInit) {
        const mintPubkey = getPublicKey("mint", network);
        const [dataAccountPubKey, ] = PublicKey.findProgramAddressSync(
            [Buffer.from("spl_staking", "utf-8"), adminKeyPair.publicKey.toBuffer(), mintPubkey.toBuffer()],
            programId
        );
        const tokenAcctLen = getMintLen([ExtensionType.TransferFeeAmount]);
        const createTokenAcctIX = SystemProgram.createAccount({
            programId: TOKEN_2022_PROGRAM_ID,
            space: tokenAcctLen,
            lamports: await connection.getMinimumBalanceForRentExemption(tokenAcctLen),
            fromPubkey: adminKeyPair.publicKey,
            newAccountPubkey: tokenAccountKeypair.publicKey
        })
        const initTokenAcctIX = createInitializeAccountInstruction(
            tokenAccountKeypair.publicKey,
            mintPubkey,
            adminKeyPair.publicKey,
            TOKEN_2022_PROGRAM_ID
        );
        console.log(MAX_FEE)
        const instructionData = Buffer.from(
            Uint8Array.of(
                0,
                ...new BN(minimumStakeAmount * 10**TOKEN_DECIMALS).toArray("le", 8),
                ...new BN(minimumLockDuration).toArray("le", 8),
                ...new BN(normalStakingApy * 10).toArray("le", 8),
                ...new BN(lockedStakingApy * 10).toArray("le", 8),
                ...new BN(earlyWithdrawalFee * 10).toArray("le", 8),
                ...new BN(FEE_BASIS_POINTS).toArray("le", 8),
                ...new BN(MAX_FEE * 10**TOKEN_DECIMALS).toArray("le", 8),
            )
        )
        const initIX = new TransactionInstruction({
            programId,
            keys: [
                { pubkey: adminKeyPair.publicKey, isSigner: true, isWritable: false },
                { pubkey: dataAccountPubKey, isSigner: false, isWritable: true },
                { pubkey: tokenAccountKeypair.publicKey, isSigner: false, isWritable: true },
                { pubkey: mintPubkey, isSigner: false, isWritable: false },
                { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
                { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }
            ],
            data: instructionData
        });
        const latestBlockHash = await connection.getLatestBlockhash('confirmed');
        const messageV0 = new TransactionMessage({
            payerKey: adminKeyPair.publicKey,
            recentBlockhash: latestBlockHash.blockhash,
            instructions: [createTokenAcctIX, initTokenAcctIX, initIX]
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
            console.log(`Transaction could not be confirmed ❌❌❌`)
        } else {
            writePublicKey(tokenAccountKeypair.publicKey, 'token_acct', network);
            writeSecretKey(tokenAccountKeypair.secretKey, 'token_acct', network);
            writePublicKey(dataAccountPubKey, 'data_acct', network);
            if (network == "devnet" || network == "localhost") {
                await mintTo(
                    connection,
                    adminKeyPair,
                    mintPubkey,
                    tokenAccountKeypair.publicKey,
                    adminKeyPair,
                    100000*10**TOKEN_DECIMALS,
                    undefined,
                    undefined,
                    TOKEN_2022_PROGRAM_ID
                )
            }
        }
    }
}

// setup(
//     "mainnet",
//     10,
//     7*24*60*60,
//     2639,
//     6057,
//     10,
//     true
// ).then((val) => console.log(val))

// let sk = Uint8Array.from(
//     JSON.parse(fs.readFileSync(`../build/spl_staking-keypair.json`) as unknown as string)
// );
// let kp = Keypair.fromSecretKey(sk);
// console.log(kp.publicKey.toString())