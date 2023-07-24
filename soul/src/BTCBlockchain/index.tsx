import { TitleComp } from "../shared";
import { BNodeInfo } from "../types";

export const BitcoinNodeInfo = ({ nodeInfo }: { nodeInfo: BNodeInfo }) => {
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
