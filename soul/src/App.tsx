import { useEffect, useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";
import { Box, Button, Modal, Typography } from "@mui/material";

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

const WalletTile = ({ walletName }: { walletName: string }) => {
    const [balance, setBalance] = useState<number>(0);
    const [recAddress, setRecAddress] = useState<string>("");

    useEffect(() => {
        async function walletInfo() {
            try {
                const info: { balance: number } = await invoke(
                    "wallet_info",
                    {
                        walletName: walletName,
                    }
                );
                setBalance(info.balance);
            } catch (error) {
                console.log(error);
            }
        }

        walletInfo();
    }, []);

    return (
        <div
            style={{
                border: "2px solid black",
                borderRadius: "10px",
                height: "150px",
                fontSize: "1.5rem",
                backgroundColor: "orange",
                cursor: "pointer",
                fontWeight: "900",
            }}
        >
            <div
                style={{ textAlign: "center", paddingTop: "3.5rem" }}
            >{`${walletName}`}</div>
            <div
                style={{ textAlign: "center", paddingTop: "1.5rem" }}
            >{`${balance} BTC`}</div>
        </div>
    );
};

const WalletScreen = ({ walletName }: { walletName: string }) => {
    const [balance, setBalance] = useState<number>(0);
    const [recAddress, setRecAddress] = useState<string>("");

    useEffect(() => {
        async function walletInfo() {
            try {
                const info: { balance: number } = await invoke(
                    "wallet_info",
                    {
                        walletName: walletName,
                    }
                );
                setBalance(info.balance);
            } catch (error) {
                console.log(error);
            }
        }
        walletInfo();
    }, []);

    return (
        <div
            style={{
                border: "1px solid black",
                borderRadius: "10px",
                height: "150px",
                fontSize: "1.5rem",
                backgroundColor: "orange",
                cursor: "pointer",
                fontWeight: "900",
            }}
        >
            <div
                style={{ textAlign: "center", paddingTop: "3.5rem" }}
            >{`${walletName}`}</div>
            <div
                style={{ textAlign: "center", paddingTop: "1.5rem" }}
            >{`${balance} BTC`}</div>
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

const style = {
    position: "absolute" as "absolute",
    top: "50%",
    left: "50%",
    transform: "translate(-50%, -50%)",
    width: 1000,
    bgcolor: "background.paper",
    border: "2px solid #000",
    boxShadow: 24,
    p: 4,
};

const generateMMC = () => {
    return "one two three four five six seven eight nine ten eleven twelve";
};

const CreateWallet = () => {
    const [mmc, setMMC] = useState<string>("");

    return (
        <Box sx={style}>
            <Typography
                id="modal-modal-title"
                variant="h6"
                component="h2"
            >
                Create Wallet
            </Typography>
            <Button
                onClick={() => {
                    const mmc = generateMMC();
                    setMMC(mmc);
                }}
            >
                Generate MMC
            </Button>
            <div
                style={{
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr 1fr 1fr",
                }}
            >
                {mmc.length ? mmc.split(" ").map((word) => {
                    return (
                        <Button style={{ cursor: "default" }}>
                            {word}
                        </Button>
                    );
                }): null}
            </div>
        </Box>
    );
};

function App() {
    const [bnodeInfo, setBNodeInfo] = useState<BNodeInfo>(
        {} as BNodeInfo
    );
    const [isCreateWalletModalOpen, setIsCreateWalletModalOpen] =
        useState<boolean>(false);
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
            <Modal
                open={isCreateWalletModalOpen}
                onClose={() => setIsCreateWalletModalOpen(false)}
                aria-labelledby="modal-modal-title"
                aria-describedby="modal-modal-description"
            >
                <CreateWallet />
            </Modal>
            <div
                style={{
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr 1fr",
                }}
            >
                {walletList.map((wallet) => (
                    <WalletTile walletName={wallet} />
                ))}
                <button
                    style={{ border: "1px solid black" }}
                    onClick={() => setIsCreateWalletModalOpen(true)}
                >
                    Create Wallet
                </button>
                {/** <LightningNodeInfo nodeInfo={lnodeInfo} />
            <div>----------------------</div>
            <BitcoinNodeInfo nodeInfo={bnodeInfo} />**/}
            </div>
        </div>
    );
}

export default App;
// <button
//     style={{ backgroundColor: "black", color: "white" }}
//     onClick={async () => {
//         try {
//             const res = await invoke("generate_address", {
//                 walletName: walletName,
//             });
//             console.log(res);
//         } catch (error) {
//             console.log(error);
//         }
//     }}
// >
//     new address
// </button>
// <button
//     style={{ backgroundColor: "black", color: "white" }}
//     onClick={() =>
//         invoke("send", {
//             sender: walletName,
//             amount: 0.5,
//             reciever: recAddress,
//         })
//     }
// >
//     Send
// </button>
// <button
//     onClick={() => console.log("recieve")}
//     style={{ backgroundColor: "black", color: "white" }}
// >
//     Recieve
// </button>
