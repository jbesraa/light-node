import { invoke } from "@tauri-apps/api/tauri";
import {
    useContext,
    useState,
    createContext,
    useEffect,
} from "react";
import { WalletInfo, WalletState } from "./types";

const WalletContext = createContext({} as WalletState);

const WalletProvider = ({ children }: any) => {
    const [walletList, setWalletList] = useState<string[]>([]);

    const loadWallets = async (): Promise<boolean> => {
        try {
            const walletList: string[] = await invoke("list_wallets");
            console.log("walletList", walletList);
            setWalletList(walletList);
            return true;
        } catch (error) {
            console.log("error fetching wallet list");
            console.log(error);
            return false;
        }
    };

    const addNewWallet = async (mmc: string): Promise<boolean> => {
        try {
            console.log("mmc", mmc);
            const res = await invoke("load_wallet_with_mmc", {
                mmc: mmc,
            });
            return true;
        } catch (error) {
            console.log("error", error);
            return false;
        }
    };

    useEffect(() => {
        loadWallets();
    }, []);

    const walletInfo = async (
        walletName: string
    ): Promise<WalletInfo> => {
        try {
            const info: WalletInfo = await invoke("wallet_info", {
                walletName: walletName,
            });
            return info;
        } catch (error) {
            console.log(error);
            return {} as WalletInfo;
        }
    };

    const generateMMC = async (): Promise<string> => {
        const m: string = await invoke("new_mmc");
        return m;
    };

    const state: WalletState = {
        loadWallets,
        walletList,
        walletInfo,
        generateMMC,
        addNewWallet,
    };

    return (
        <WalletContext.Provider value={state}>
            {children}
        </WalletContext.Provider>
    );
};

export const useWalletContext = () => useContext(WalletContext);

export default WalletProvider;
