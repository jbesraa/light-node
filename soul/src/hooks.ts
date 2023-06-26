import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState } from "react";
import { BNodeInfo, LNodeInfo } from "./types";

export const useWalletList = (): string[] => {
    const [walletList, setWalletList] = useState<string[]>([]);

    useEffect(() => {
        const loadWallets = async () => {
            try {
                const walletList: string[] = await invoke(
                    "list_wallets"
                );
                console.log("walletList", walletList);
                setWalletList(walletList);
            } catch (error) {
                console.log(error);
            }
        };
        loadWallets();
    }, []);

    return walletList;
};

export const useLNInfo = (): LNodeInfo => {
    const [lnodeInfo, setLNodeInfo] = useState<LNodeInfo>(
        {} as LNodeInfo
    );

    useEffect(() => {
        const loadWallets = async () => {
            try {
                const lninfo: LNodeInfo = await invoke("get_data");
                setLNodeInfo(lninfo);
            } catch (error) {
                console.log(error);
            }
        };
        loadWallets();
    }, []);

    return lnodeInfo;
};

export const useBlockchainInfo = (): BNodeInfo => {
    const [bnodeInfo, setBNodeInfo] = useState<BNodeInfo>(
        {} as BNodeInfo
    );

    useEffect(() => {
        const loadWallets = async () => {
            try {
                const bcinfo: BNodeInfo = await invoke(
                    "get_blockchain_info"
                );
                setBNodeInfo(bcinfo);
            } catch (error) {
                console.log(error);
            }
        };
        loadWallets();
    }, []);

    return bnodeInfo;
};
