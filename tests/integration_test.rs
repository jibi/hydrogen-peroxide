mod utils;
use utils::*;

#[test]
fn test_echo_server_single_queue() {
    let mut dev = init_tun();
    let _xsk = init_xsk(&dev, vec![0], 1, false);

    test_echo_server(&mut dev, 0);
}

#[test]
fn test_echo_server_multi_queue() {
    let mut dev = init_tun();
    let _xsk = init_xsk(&dev, vec![0, 1], 1, false);

    for i in 0..2 {
        test_echo_server(&mut dev, i);
    }
}

#[test]
fn test_echo_server_multi_socks_per_queue() {
    let mut dev = init_tun();
    let _xsk = init_xsk(&dev, vec![0], 2, false);

    test_echo_server(&mut dev, 0);
    test_echo_server_odd_src_port(&mut dev, 0);
}

#[test]
fn test_echo_server_multi_queue_multi_socks_per_queue() {
    let mut dev = init_tun();
    let _xsk = init_xsk(&dev, vec![0, 1], 2, false);

    for i in 0..2 {
        test_echo_server(&mut dev, i);
        test_echo_server_odd_src_port(&mut dev, i);
    }
}

#[test]
fn test_echo_server_single_queue_repeated() {
    let mut dev = init_tun();
    let _xsk = init_xsk(&dev, vec![0, 1], 2, true);

    test_echo_server_repeated(&mut dev, 0);
}
