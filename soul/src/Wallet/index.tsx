import { Box, Button, TextField, Typography } from "@mui/material";
import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState } from "react";
import { ValidateMMCProps } from "../types";
import { useWalletContext } from "../WalletContext";

export const CreateWallet = () => {
    const walletController = useWalletContext();
    const [mmc, setMMC] = useState<string>("");
    const [showMMC, setShowMMC] = useState<boolean>(false);
    const [showValidationInputs, setShowValidationInputs] =
        useState<boolean>(false);

    return (
        <Box sx={style}>
            <Typography
                style={{ textAlign: "center" }}
                id="modal-modal-title"
                variant="h6"
                component="h2"
            >
                Create Wallet
            </Typography>

            <div
                style={{
                    paddingTop: "1rem",
                    display: "grid",
                    justifyItems: "center",
                }}
            >
                {!showMMC && (
                    <Button
                        onClick={async () => {
                            const mmc =
                                await walletController.generateMMC();
                            setMMC(mmc);
                            setShowMMC(true);
                        }}
                        disabled={showMMC}
                    >
                        Generate MMC
                    </Button>
                )}
            </div>
            <div
                style={{
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr 1fr 1fr",
                }}
            >
                {showMMC && !showValidationInputs && (
                    <div>
                        <div>
                            write down your mmc securly before you
                            click next! this is your password!
                        </div>

                        {mmc.split(" ").map((word, i) => {
                            return (
                                <Button
                                    key={i}
                                    variant="contained"
                                    style={{
                                        padding: "1rem",
                                        margin: "0.5rem",
                                        cursor: "default",
                                    }}
                                >
                                    {word}
                                </Button>
                            );
                        })}
                        <div
                            style={{
                                display: "grid",
                                paddingTop: "1rem",
                            }}
                        >
                            <Button
                                variant="outlined"
                                disabled={!showMMC}
                                onClick={() =>
                                    setShowValidationInputs(true)
                                }
                            >
                                Next
                            </Button>
                        </div>
                    </div>
                )}
            </div>
            {showValidationInputs && (
                <ValidateWalletMMC words={mmc} />
            )}
        </Box>
    );
};

const ValidateWalletMMC = (props: ValidateMMCProps) => {
    const walletController = useWalletContext();
    const [word1, setWord1] = useState<string>("one");
    const [word2, setWord2] = useState<string>("two");
    const [word3, setWord3] = useState<string>("three");
    const [word4, setWord4] = useState<string>("four");
    const [word5, setWord5] = useState<string>("five");
    const [word6, setWord6] = useState<string>("six");
    const [word7, setWord7] = useState<string>("seven");
    const [word8, setWord8] = useState<string>("eight");
    const [word9, setWord9] = useState<string>("nine");
    const [word10, setWord10] = useState<string>("ten");
    const [word11, setWord11] = useState<string>("eleven");
    const [word12, setWord12] = useState<string>("twelve");

    const validateMMC = async () => {
        await walletController.addNewWallet(props?.words);
        await walletController.loadWallets();
    };

    return (
        <div>
            <div
                style={{
                    display: "grid",
                    gridTemplateColumns: "1fr 1fr 1fr 1fr",
                }}
            >
                <TextField
                    placeholder="Word 1"
                    value={word1}
                    onChange={(e) => setWord1(e.target.value)}
                    variant="outlined"
                    style={{
                        padding: "1rem",
                        margin: "0.5rem",
                        cursor: "default",
                    }}
                />
                <TextField
                    placeholder="Word 2"
                    value={word2}
                    onChange={(e) => setWord2(e.target.value)}
                    variant="outlined"
                    style={{
                        padding: "1rem",
                        margin: "0.5rem",
                        cursor: "default",
                    }}
                />
                <TextField
                    placeholder="Word 3"
                    variant="outlined"
                    value={word3}
                    onChange={(e) => setWord3(e.target.value)}
                    style={{
                        padding: "1rem",
                        margin: "0.5rem",
                        cursor: "default",
                    }}
                />
                <TextField
                    placeholder="Word 4"
                    value={word4}
                    onChange={(e) => setWord4(e.target.value)}
                    variant="outlined"
                    style={{
                        padding: "1rem",
                        margin: "0.5rem",
                        cursor: "default",
                    }}
                />
                <TextField
                    placeholder="Word 5"
                    value={word5}
                    onChange={(e) => setWord5(e.target.value)}
                    variant="outlined"
                    style={{
                        padding: "1rem",
                        margin: "0.5rem",
                        cursor: "default",
                    }}
                />
                <TextField
                    value={word6}
                    onChange={(e) => setWord6(e.target.value)}
                    placeholder="Word 6"
                    variant="outlined"
                    style={{
                        padding: "1rem",
                        margin: "0.5rem",
                        cursor: "default",
                    }}
                />
                <TextField
                    value={word7}
                    onChange={(e) => setWord7(e.target.value)}
                    placeholder="Word 7"
                    variant="outlined"
                    style={{
                        padding: "1rem",
                        margin: "0.5rem",
                        cursor: "default",
                    }}
                />
                <TextField
                    placeholder="Word 8"
                    value={word8}
                    onChange={(e) => setWord8(e.target.value)}
                    variant="outlined"
                    style={{
                        padding: "1rem",
                        margin: "0.5rem",
                        cursor: "default",
                    }}
                />
                <TextField
                    placeholder="Word 9"
                    value={word9}
                    onChange={(e) => setWord9(e.target.value)}
                    variant="outlined"
                    style={{
                        padding: "1rem",
                        margin: "0.5rem",
                        cursor: "default",
                    }}
                />
                <TextField
                    placeholder="Word 10"
                    value={word10}
                    onChange={(e) => setWord10(e.target.value)}
                    variant="outlined"
                    style={{
                        padding: "1rem",
                        margin: "0.5rem",
                        cursor: "default",
                    }}
                />
                <TextField
                    placeholder="Word 11"
                    value={word11}
                    onChange={(e) => setWord11(e.target.value)}
                    variant="outlined"
                    style={{
                        padding: "1rem",
                        margin: "0.5rem",
                        cursor: "default",
                    }}
                />
                <TextField
                    placeholder="Word 12"
                    value={word12}
                    onChange={(e) => setWord12(e.target.value)}
                    variant="outlined"
                    style={{
                        padding: "1rem",
                        margin: "0.5rem",
                        cursor: "default",
                    }}
                />
            </div>

            <Button
                variant="outlined"
                disabled={false}
                style={{ width: "100%" }}
                onClick={validateMMC}
            >
                Next
            </Button>
        </div>
    );
};

export const WalletTile = ({
    walletName,
    onSelect,
}: {
    onSelect: (wn: string) => void;
    walletName: string;
}) => {
    let balance = 0;

    return (
        <div
            onClick={() => onSelect(walletName)}
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

export interface TransactionDetails {
    transaction?: any;
    txid: string;
    received: number;
    sent: number;
    fee?: any;
    confirmation_time?: any;
}

export const WalletScreen = ({
    walletName,
}: {
    walletName: string;
}) => {
    const [balance, setBalance] = useState<number>(0);
    const [txcount, setTxcount] = useState<number>(0);
    const [txList, setTxList] = useState<TransactionDetails[]>([]);
    const [immatureBalance, setImmatureBalance] = useState<number>(0);


    useEffect(() => {
        async function walletInfo() {
            try {
                const info: {
                    balance: number;
                    walletname: string;
                    txcount: number;
                } = await invoke("wallet_info", {
                    walletName: walletName,
                });
                    console.log("info", info);
                const txs: any = await invoke("list_txs", {
                    walletName: walletName,
                });
                setTxList(txs);
                setBalance(info.balance);
                setImmatureBalance(info.immature_balance);
                setTxcount(info.txcount);
            } catch (error) {
                console.log(error);
            }
        }

        walletInfo();
    }, [walletName]);

    const generateToAddress = async (walletName: string) => {
        try {
            const txs: any = await invoke("generate_to_address", {
                walletName: walletName,
            });
        } catch (error) {
            return false;
        }
    };

    return (
        <div
            style={{
                border: "1px solid black",
                borderRadius: "10px",
                fontSize: "1.5rem",
                cursor: "pointer",
                fontWeight: "900",
            }}
        >
            <div
                style={{ textAlign: "center", paddingTop: "3.5rem" }}
            >{`${walletName}`}</div>
            <div
                style={{ textAlign: "center", paddingTop: "1.5rem" }}
            >{`Balance: ${balance} BTC`}</div>
            <div
                style={{ textAlign: "center", paddingTop: "1.5rem" }}
            >{`Immature Balance: ${immatureBalance} BTC`}</div>
            <div
                style={{ textAlign: "center", paddingTop: "1.5rem" }}
            >{`Transactions Count: ${txcount}`}</div>
            <div
                style={{ textAlign: "center", paddingTop: "1.5rem" }}
            >
                {txList.length}
            </div>
            <Button onClick={() => generateToAddress(walletName)}>
                MINE TO SELF
            </Button>
        </div>
    );
};

const style = {
    position: "absolute" as "absolute",
    top: "50%",
    left: "50%",
    transform: "translate(-50%, -50%)",
    width: 2000,
    // height: 300,
    bgcolor: "background.paper",
    borderRadius: "1rem",
    boxShadow: 24,
    p: 4,
};
