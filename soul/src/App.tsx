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

const WalletComp = ({ walletName }: { walletName: string }) => {
    const [balance, setBalance] = useState<number>(0);
    const [recAddress, setRecAddress] = useState<string>("");

    useEffect(() => {
        async function walletInfo() {
            try {
                console.log("walletName", walletName);
                const info: { balance: number } = await invoke(
                    "wallet_info",
                    {
                        walletName: walletName,
                    }
                );
                setBalance(info.balance);
                console.log("info", info);
            } catch (error) {
                console.log(error);
            }
        }
        walletInfo();
    }, []);

    return (
        <div
            style={{
                fontSize: "1.5rem",
                fontWeight: "900",
            }}
        >
            {`Name: ${walletName}`}
            {`Balance: ${balance}`}
            <input
                onChange={(i) => setRecAddress(i.target.value)}
                value={recAddress}
            />
            <button
                style={{ backgroundColor: "black", color: "white" }}
                onClick={async () => {
                try {                     const res = await invoke("generate_address", {
walletName: walletName,
});
                console.log(res)
                } catch(error) {
                    console.log(error)
                }                }}
            >
                new address
            </button>
            <button
                style={{ backgroundColor: "black", color: "white" }}
                onClick={() =>
                    invoke("send", {
                        sender: walletName,
                        amount: 0.5,
                        reciever: recAddress,
                    })
                }
            >
                Send
            </button>
            <button
                onClick={() => console.log("recieve")}
                style={{ backgroundColor: "black", color: "white" }}
            >
                Recieve
            </button>
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
    const [walletList, setWalletList] = useState<string[]>([]);
    const [lnodeInfo, setLNodeInfo] = useState<LNodeInfo>(
        {} as LNodeInfo
    );

    useEffect(() => {
        async function greet() {
            try {
                // const lninfo: LNodeInfo = await invoke("get_data");
                // const bcinfo: BNodeInfo = await invoke(
                //     "get_blockchain_info"
                // );
                const walletList: string[] = await invoke(
                    "list_wallets"
                );
                console.log("walletList", walletList);
                setWalletList(walletList);
                // setLNodeInfo(lninfo);
                // setBNodeInfo(bcinfo);
            } catch (error) {
                console.log(error);
            }
        }
        greet();
    }, []);

    return (
        <div>
            <h1>Soul</h1>
            <div style={{ fontSize: "2rem" }}> Balance: </div>
            <button onClick={() => console.log("connect")}>
                List Wallets
            </button>
            <div>
                {walletList.map((wallet) => (
                    <WalletComp walletName={wallet} />
                ))}
            </div>
            {/** <LightningNodeInfo nodeInfo={lnodeInfo} />
            <div>----------------------</div>
            <BitcoinNodeInfo nodeInfo={bnodeInfo} />**/}
        </div>
    );
}

export default App;
