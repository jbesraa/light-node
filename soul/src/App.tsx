import { Modal } from "@mui/material";
import { useState } from "react";
import "./App.css";
import { useWalletList } from "./hooks";
import { CreateWallet, WalletScreen, WalletTile } from "./Wallet";

const App = () => {
    const [isCreateWalletModalOpen, setIsCreateWalletModalOpen] =
        useState<boolean>(false);
    const [selectedWallet, setSelectedWallet] = useState<string>("");

    const walletList = useWalletList();

    const onWalletSelect = (walletName: string) => {
        setSelectedWallet(walletName);
    };

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
                {walletList.map((wallet, i) => (
                    <WalletTile
                        key={i}
                        walletName={wallet}
                        onSelect={onWalletSelect}
                    />
                ))}
                <button
                    style={{ border: "1px solid black" }}
                    onClick={() => setIsCreateWalletModalOpen(true)}
                >
                    Create Wallet
                </button>
            </div>
            {selectedWallet && (
                <div>
                    <WalletScreen walletName={selectedWallet} />
                </div>
            )}
        </div>
    );
};

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
