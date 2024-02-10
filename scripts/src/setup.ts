import {
    Connection,
    Keypair, PublicKey, Signer,
    SystemProgram,
    SYSVAR_RENT_PUBKEY,
    Transaction,
    TransactionInstruction, TransactionMessage, VersionedTransaction
} from "@solana/web3.js";
import BN from "bn.js";
import {
    AccountLayout,
    //ASSOCIATED_TOKEN_PROGRAM_ID,
    getOrCreateAssociatedTokenAccount,
    createInitializeAccountInstruction,
    createMint,
    mintTo,
    TOKEN_PROGRAM_ID
} from "@solana/spl-token";
import fs from "fs";
// import { struct, u8 } from '@solana/buffer-layout';
// import { publicKey, u64 } from '@solana/buffer-layout-utils';

const PROGRAM_ID = "7iPzfTTkxYbEZy8JQLfsafApbzw5m9JYE7Amt7zeEDST";
const TOKEN_DECIMALS = 6;
const LOCALNET_CONNECTION_URL = "http://127.0.0.1:8899";
const DEVNET_CONNECTION_URL = "https://few-yolo-sky.solana-devnet.quiknode.pro/3852afeceff67333bb3ccaa4172b8f9e5df67e23/";
const MAINNET_CONNECTION_URL = "https://solana-mainnet.g.alchemy.com/v2/a0Xic8r2YTu7uJ-O-Gn27SgmDTKaelhL";

// interface ContractData {
//     isInitialized: number,
//     adminPubkey: PublicKey,
//     stakeTokenMint: PublicKey,
//     stakeTokenAccount: PublicKey,
//     minimumStakeAmount: bigint,
//     minimumLockDuration: bigint,
//     normalStakingApy: bigint,
//     lockedStakingApy: bigint,
//     earlyWithdrawalFee: bigint,
//     totalStaked: bigint,
//     totalEarned: bigint
// }
//
// const CONTRACT_DATA_LAYOUT = struct<ContractData>([
//     u8('isInitialized'),
//     publicKey('adminPubkey'),
//     publicKey('stakeTokenMint'),
//     publicKey('stakeTokenAccount'),
//     u64('minimumStakeAmount'),
//     u64('minimumLockDuration'),
//     u64('normalStakingApy'),
//     u64('lockedStakingApy'),
//     u64('earlyWithdrawalFee'),
//     u64('totalStaked'),
//     u64('totalEarned')
// ])


const getPublicKey = (name: string, network: string = "localnet") =>
    new PublicKey(
        JSON.parse(fs.readFileSync(`./keys/${network}/${name}_pub.json`) as unknown as string)
    );

const getPrivateKey = (name: string, network: string = "localnet") =>
    Uint8Array.from(
        JSON.parse(fs.readFileSync(`./keys/${network}/${name}.json`) as unknown as string)
    );

const getKeypair = (name: string, network: string = "localnet") =>
    new Keypair({
        publicKey: getPublicKey(name, network).toBytes(),
        secretKey: getPrivateKey(name, network),
    });

const writePublicKey = (publicKey: PublicKey, name: string, network: string = "localnet") => {
    const path = `./keys/${network}/${name}_pub.json`
    console.log(`Writing Public Key To: ${path}`)
    fs.writeFileSync(
        path,
        JSON.stringify(publicKey.toString())
    );
};

const writeSecretKey = (secretKey: Uint8Array, name: string, network: string = "localnet") => {
    const path = `./keys/${network}/${name}.json`
    console.log(`Writing Secret Key To: ${path}`)
    fs.writeFileSync(
        path,
        `[${secretKey.toString()}]`
    );
};

const setupMint = async (
    name: string,
    network: string,
    connection: Connection,
    clientKeypair: Signer,
    mintTokens: boolean
) => {
    console.log(`Creating Mint ${name}...`);
    let mintKeyPair: Keypair;
    try {
        mintKeyPair = getKeypair("mint", network);
    } catch {
        mintKeyPair = new Keypair();
        await createMint(
            connection,
            {
                publicKey: clientKeypair.publicKey,
                secretKey: clientKeypair.secretKey,
            },
            clientKeypair.publicKey,
            null,
            TOKEN_DECIMALS,
            mintKeyPair
        );
        writePublicKey(mintKeyPair.publicKey, "mint", network);
        writeSecretKey(mintKeyPair.secretKey, "mint", network);
    }
    // For Devnet [ Create Token Accounts and Mint Tokens ]
    if (mintTokens) {
        console.log('Mintinggggg')
        const associatedToken = await getOrCreateAssociatedTokenAccount(
            connection,
            clientKeypair,
            mintKeyPair.publicKey,
            new PublicKey("Fr1vWrS8YYFVjHx9UkzL5gFCUSP2tVBH3aGSK9ZTi7LP")
        )
        await mintTo(
            connection,
            clientKeypair,
            mintKeyPair.publicKey,
            associatedToken.address,
            clientKeypair,
            2000*10**TOKEN_DECIMALS
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
) => {
    let rpc;
    if (network == "devnet") {
        rpc =DEVNET_CONNECTION_URL
    } else if (network == "localnet") {
        rpc = LOCALNET_CONNECTION_URL
    } else {
        rpc = MAINNET_CONNECTION_URL
    }
    const connection = new Connection(rpc, "confirmed");
    const programId = new PublicKey(PROGRAM_ID);
    let tokenAccountKeypair = new Keypair();
    let adminKeyPair: Keypair;
    try {
        adminKeyPair = getKeypair("admin", network);
    } catch {
        adminKeyPair = new Keypair();
        writePublicKey(adminKeyPair.publicKey, "admin", network);
        writeSecretKey(adminKeyPair.secretKey, "admin", network);
    }
    if (network == "devnet" || network == "localnet") {
        await setupMint(
            "Libra",
            network,
            connection,
            adminKeyPair,
            true
        )
    }
    const mintPubkey = getPublicKey("mint", network);
    const [dataAccountPubKey, ] = PublicKey.findProgramAddressSync(
        [Buffer.from("spl_staking", "utf-8"), adminKeyPair.publicKey.toBuffer(), mintPubkey.toBuffer()],
        programId
    );
    const [dataAccount,] = PublicKey.findProgramAddressSync(
        [Buffer.from("spl_staking", "utf-8"), adminKeyPair.publicKey.toBuffer(), mintPubkey.toBuffer()],
        programId
    );
    const createTokenAcctIX = SystemProgram.createAccount({
        programId: TOKEN_PROGRAM_ID,
        space: AccountLayout.span,
        lamports: await connection.getMinimumBalanceForRentExemption(AccountLayout.span),
        fromPubkey: adminKeyPair.publicKey,
        newAccountPubkey: tokenAccountKeypair.publicKey
    })
    const initTokenAcctIX = createInitializeAccountInstruction(
        tokenAccountKeypair.publicKey,
        mintPubkey,
        adminKeyPair.publicKey
    );
    const instructionData = Buffer.from(
        Uint8Array.of(
            0,
            ...new BN(minimumStakeAmount * 10**TOKEN_DECIMALS).toArray("le", 8),
            ...new BN(minimumLockDuration).toArray("le", 8),
            ...new BN(normalStakingApy * 10).toArray("le", 8),
            ...new BN(lockedStakingApy * 10).toArray("le", 8),
            ...new BN(earlyWithdrawalFee * 10).toArray("le", 8)
        )
    )
    const initIX = new TransactionInstruction({
        programId,
        keys: [
            { pubkey: adminKeyPair.publicKey, isSigner: true, isWritable: false },
            { pubkey: dataAccountPubKey, isSigner: false, isWritable: true },
            { pubkey: tokenAccountKeypair.publicKey, isSigner: false, isWritable: true },
            { pubkey: mintPubkey, isSigner: false, isWritable: false },
            { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
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
        await mintTo(
            connection,
            adminKeyPair,
            mintPubkey,
            tokenAccountKeypair.publicKey,
            adminKeyPair,
            100000*10**TOKEN_DECIMALS
        )
    }
}

setup(
    "devnet",
    100,
    24*60*60,
    500,
    700,
    1
).then((val) => console.log(val))