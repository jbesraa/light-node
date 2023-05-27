import { useEffect, useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

const TitleComp = ({ t }: { t: string }) => {
    return (
        <div
            style={{
                textDecorationLine: "underline",
                fontWeight: "900",
            }}
        >
            {t}
        </div>
    );
};

const LightningNodeInfo = ({ nodeInfo }: { nodeInfo: LNodeInfo }) => {
    const {
        pubkey,
        network,
        port,
        node_name,
        announced_listen_addr,
        num_usable_channels,
        num_channels,
        local_balance_msat,
        num_peers,
    } = nodeInfo;
    return (
        <>
            <TitleComp t={"Lightning Node Info"} />
            <div>Pubkey: {pubkey}</div>
            <div>Network: {network}</div>
            <div>Port: {port}</div>
            <div>Node Name: {node_name}</div>
            <div>Announced Listen Addr: {announced_listen_addr}</div>
            <div>Num Usable Channels: {num_usable_channels}</div>
            <div>Num Channels: {num_channels}</div>
            <div>Local Balance Msat: {local_balance_msat}</div>
            <div>Num Peers: {num_peers}</div>
        </>
    );
};

const BitcoinNodeInfo = ({ nodeInfo }: { nodeInfo: BNodeInfo }) => {
    const { latest_height, latest_blockhash, chain } = nodeInfo;
    return (
        <>
            <TitleComp t={"Bitcoin Node Info"} />
            <div>Latest Block Height: {latest_height}</div>
            <div>Latest Blockhash: {latest_blockhash}</div>
            <div>Chain: {chain}</div>
        </>
    );
};

interface BNodeInfo {
    latest_height: number;
    latest_blockhash: string;
    chain: string;
}

interface LNodeInfo {
    pubkey: string;
    network: string;
    port: number;
    node_name: String;
    announced_listen_addr: String;
    num_usable_channels: number;
    num_channels: number;
    local_balance_msat: number;
    num_peers: number;
}

function App() {
    const [bnodeInfo, setBNodeInfo] = useState<BNodeInfo>(
        {} as BNodeInfo
    );
    const [lnodeInfo, setLNodeInfo] = useState<LNodeInfo>(
        {} as LNodeInfo
    );

    useEffect(() => {
        async function greet() {
            try {
                const lninfo: LNodeInfo = await invoke("get_data");
                const bcinfo: BNodeInfo = await invoke(
                    "get_blockchain_info"
                );
                setLNodeInfo(lninfo);
                setBNodeInfo(bcinfo);
            } catch (error) {
                console.log(error);
            }
        }
        greet();
    }, []);

    return (
        <div>
            <h1>Soul</h1>
            <LightningNodeInfo nodeInfo={lnodeInfo} />
            <div>----------------------</div>
            <BitcoinNodeInfo nodeInfo={bnodeInfo} />
        </div>
    );
}

export default App;
