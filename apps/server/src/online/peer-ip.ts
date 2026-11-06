import { isIP } from "node:net";
/** One literal address only: no CIDR, forwarded chain, port, hostname or IPv6 scope. */
export function normalizeIp(raw: string): string | null {
    const ip = raw.trim().toLowerCase();
    if (ip.includes("%") || !isIP(ip))
        return null;
    if (ip.startsWith("::ffff:") && isIP(ip.slice(7)) === 4)
        return ip.slice(7);
    if (isIP(ip) === 4)
        return ip;
    return new URL(`http://[${ip}]/`).hostname.slice(1, -1);
}
export function proxyPeers(values: readonly string[]): Set<string> {
    const peers = new Set<string>();
    for (const value of values) {
        const ip = normalizeIp(value);
        if (!ip)
            throw new Error("Invalid trusted proxy IP");
        peers.add(ip);
    }
    return peers;
}
