export interface BNodeInfo {
    latest_height: number;
    latest_blockhash: string;
    chain: string;
}

export interface LNodeInfo {
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


export interface ValidateMMCProps {
    words: string;
}

export interface WalletState {
    loadWallets: () => Promise<boolean>;
    walletList: string[];
    walletInfo: (walletName: string) => Promise<WalletInfo>;
    generateMMC: () => Promise<string>;
    addNewWallet: (mmc: string) => Promise<boolean>;
}

export interface WalletInfo {
  balance: number;
}


