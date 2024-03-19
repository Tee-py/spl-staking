import {Keypair, PublicKey} from "@solana/web3.js";
import fs from "fs";
import {decode} from "bs58";
import {DEVNET_CONNECTION_URL, LOCALNET_CONNECTION_URL, MAINNET_CONNECTION_URL} from "./constant";

export const getPublicKey = (name: string, network: string = "localnet") =>
    new PublicKey(
        JSON.parse(fs.readFileSync(`./keys/${network}/${name}_pub.json`) as unknown as string)
    );

export const getPrivateKey = (name: string, network: string = "localnet") =>
    Uint8Array.from(
        JSON.parse(fs.readFileSync(`./keys/${network}/${name}.json`) as unknown as string)
    );

export const getKeypair = (name: string, network: string = "localnet", isSecret?: boolean) => {
    if (isSecret) {
        const decoded = decode(JSON.parse(fs.readFileSync(`./keys/${network}/${name}.json`) as unknown as string));
        return Keypair.fromSecretKey(decoded);
    }
    return new Keypair({
        publicKey: getPublicKey(name, network).toBytes(),
        secretKey: getPrivateKey(name, network),
    });
}

export const writePublicKey = (publicKey: PublicKey, name: string, network: string = "localnet") => {
    const path = `./keys/${network}/${name}_pub.json`
    console.log(`Writing Public Key To: ${path}`)
    fs.writeFileSync(
        path,
        JSON.stringify(publicKey.toString())
    );
};

export const writeSecretKey = (secretKey: Uint8Array, name: string, network: string = "localnet") => {
    const path = `./keys/${network}/${name}.json`
    console.log(`Writing Secret Key To: ${path}`)
    fs.writeFileSync(
        path,
        `[${secretKey.toString()}]`
    );
};

export const getCluster = (network: string) => {
    let cluster;
    if (network == "devnet") {
        cluster =DEVNET_CONNECTION_URL
    } else if (network == "localnet") {
        cluster = LOCALNET_CONNECTION_URL
    } else {
        cluster = MAINNET_CONNECTION_URL
    }
    return cluster
}