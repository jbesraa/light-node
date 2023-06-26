import { TitleComp } from "../shared";
import { LNodeInfo } from "../types";

export const LightningNodeInfo = ({ nodeInfo }: { nodeInfo: LNodeInfo }) => {
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


