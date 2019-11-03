// Copyright (C) 2020 Gilberto "jibi" Bertin <me@jibi.io>
//
// This file is part of hydrogen peroxyde.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#include <stddef.h>

#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <linux/udp.h>
#include <linux/in.h>
#include <linux/bpf.h>

#include <bpf_helpers.h>
#include <bpf_endian.h>

#include "utils.h"

SINGLE_VAL_MAP(socks_per_queue_map, u32);
SINGLE_VAL_MAP(bind_addr_map, u32);
SINGLE_VAL_MAP(bind_port_map, u16);

struct bpf_map_def SEC("maps") xsks_map = {
	.type = BPF_MAP_TYPE_XSKMAP,
	.key_size = sizeof(i32),
	.value_size = sizeof(i32),
	.max_entries = 1024,
};

static inline
i32 redirect_to_xsk(struct xdp_md *xdp, u16 sport) {
	u32 socks_per_queue = get_val(u32, socks_per_queue_map);
	u32 index = xdp->rx_queue_index * socks_per_queue + (sport % socks_per_queue);

	return bpf_redirect_map(&xsks_map, index, XDP_PASS);
}

SEC("xdp/prog")
i32 xdp_sock_prog(struct xdp_md *xdp) {
	void *data = (void *)(u64)xdp->data;
	void *data_end = (void *)(u64)xdp->data_end;

	struct ethhdr *eth = (struct ethhdr*)data;
	if (eth + 1 > (struct ethhdr *)data_end)
		return XDP_ABORTED;

	if (eth->h_proto == bpf_htons(ETH_P_IP)) {
		struct iphdr *ip = (struct iphdr *)(eth + 1);
		if (ip + 1 > (struct iphdr *)data_end)
			return XDP_ABORTED;

		if (ip->daddr != bpf_htonl(get_val(u32, bind_addr_map)))
			return XDP_PASS;
		if (ip->protocol != IPPROTO_UDP)
			return XDP_PASS;

		struct udphdr *udp = (struct udphdr *)(ip + 1);
		if (udp + 1 > (struct udphdr *)data_end)
			return XDP_ABORTED;

		if (udp->dest != bpf_htons(get_val(u16, bind_port_map)))
			return XDP_PASS;

		return redirect_to_xsk(xdp, udp->source);
	} else if (eth->h_proto == bpf_htons(ETH_P_ARP)) {
		return redirect_to_xsk(xdp, 0);
	}

	return XDP_PASS;
}

char _license[] SEC("license") = "AGPL v3";

