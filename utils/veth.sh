#!/bin/sh

# this is just for testing purposes

sudo ip link add veth1 type veth peer name veth2

sudo ip addr add 198.18.0.1/24 dev veth1
sudo ip addr add 198.18.0.2/24 dev veth2

sudo ip addr add fc00::198.18.0.1/120 dev veth1
sudo ip addr add fc00::198.18.0.2/120 dev veth2

sudo ip link set veth1 up
sudo ip link set veth2 up

sudo ip route add 198.18.3.0/24 dev veth1
sudo ip route add fc00::198.18.3.0/120 dev veth1

#sudo arp -s 198.18.3.2 $(ip a s veth2 | grep ether | awk '{print $2}')
#sudo ip -6 neigh add to fc00::198.18.3.2 lladdr $(ip a s veth2 | grep ether | awk '{print $2}') dev veth1
