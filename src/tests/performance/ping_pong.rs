use std::{net::SocketAddr, time::Duration};

use recorder::enable_simple_recorder;

use crate::{
    protocol::{message::Message, payload::Nonce},
    setup::node::{Action, Node},
    tools::{
        metrics::{
            recorder,
            tables::{duration_as_ms, RequestStats, RequestsTable},
        },
        synthetic_node::SyntheticNode,
    },
};

const PINGS: u16 = 1000;
const METRIC_LATENCY: &str = "ping_perf_latency";

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn throughput() {
    // ZG-PERFORMANCE-001, Ping-Pong latency
    //
    // The node behaves as expected under load from other peers.
    //
    // We test the overall performance of a node's Ping-Pong latency.
    //
    // Note: This test does not assert any requirements, but requires manual inspection
    //       of the results table. This is because the results will rely on the machine
    //       running the test.
    //
    // ZCashd: Performs well.
    //
    // Zebra: Starts exceeding the response timeout from 200 onwards. Occasionally triggers a possible
    //        internal bug, logs "thread 'tokio-runtime-worker' panicked at 'internal error: entered unreachable code'".
    //        See [SyntheticNode::perform_handshake()] comments for more information.
    //
    // Example test result (with percentile latencies):
    //  *NOTE* run with `cargo test --release tests::performance::ping_pong::throughput -- --nocapture`
    //
    //  ZCashd
    //
    // ┌───────┬──────────┬──────────┬──────────┬──────────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────────┬──────────┬────────────┐
    // │ peers │ requests │ min (ms) │ max (ms) │ std dev (ms) │ 10% (ms) │ 50% (ms) │ 75% (ms) │ 90% (ms) │ 99% (ms) │ completion % │ time (s) │ requests/s │
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │      1│      1000│         0│        50│             2│         0│         0│         0│         0│         0│        100.00│      0.18│     5439.45│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     10│      1000│         0│       100│             9│         0│         0│         0│         0│        50│        100.00│      1.63│     6120.19│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     20│      1000│         0│       102│             8│         0│         0│         0│         0│        51│        100.00│      1.82│    10973.24│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     30│      1000│         0│       102│             8│         1│         1│         1│         1│        51│        100.00│      2.20│    13656.85│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     40│      1000│         0│       104│             7│         1│         1│         1│         1│         2│        100.00│      2.33│    17182.12│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     50│      1000│         0│       102│             5│         1│         2│         2│         2│         2│        100.00│      2.55│    19608.57│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     60│      1000│         0│       105│             8│         2│         2│         2│         2│         3│        100.00│      3.38│    17776.77│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     70│      1000│         0│        42│             1│         2│         3│         3│         3│         3│        100.00│      3.27│    21392.13│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     80│      1000│         0│       110│             6│         3│         3│         3│         3│         4│        100.00│      4.11│    19445.54│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     90│      1000│         0│       108│             4│         3│         4│         4│         4│         5│        100.00│      4.49│    20024.84│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │    100│      1000│         0│       101│             2│         4│         4│         4│         4│         5│        100.00│      4.71│    21230.18│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │    200│      1000│         0│       100│             2│         7│         8│         9│         9│        10│        100.00│      9.12│    21931.53│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │    300│      1000│         0│       102│             2│        12│        13│        13│        14│        15│        100.00│     13.46│    22292.24│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │    500│      1000│         0│        95│             3│        19│        20│        21│        22│        24│        100.00│     21.25│    23532.20│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │    750│      1000│         0│       100│             3│        29│        31│        32│        33│        35│        100.00│     31.67│    23681.60│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │    800│      1000│         0│       102│             4│        30│        34│        35│        36│        38│        100.00│     34.39│    23264.83│
    // └───────┴──────────┴──────────┴──────────┴──────────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────────┴──────────┴────────────┘
    //
    //  zebra
    // ┌───────┬──────────┬──────────┬──────────┬──────────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────────┬──────────┬────────────┐
    // │ peers │ requests │ min (ms) │ max (ms) │ std dev (ms) │ 10% (ms) │ 50% (ms) │ 75% (ms) │ 90% (ms) │ 99% (ms) │ completion % │ time (s) │ requests/s │
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │      1│      1000│         0│         1│             1│         0│         0│         0│         1│         1│        100.00│      0.83│     1198.47│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     10│      1000│         0│        13│             1│         0│         1│         1│         1│         3│        100.00│      1.30│     7716.68│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     20│      1000│         0│       106│             6│         0│         1│         1│         2│        27│        100.00│      2.15│     9301.11│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     30│      1000│         0│       187│            10│         0│         1│         1│         3│        48│        100.00│      3.21│     9356.83│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     40│      1000│         0│       211│            15│         0│         1│         1│         2│        81│        100.00│      4.19│     9551.86│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     50│      1000│         0│       456│            21│         0│         1│         1│         1│       105│        100.00│      5.22│     9571.41│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     60│      1000│         0│       351│            23│         0│         1│         1│         2│       126│        100.00│      6.20│     9680.17│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     70│      1000│         0│       384│            27│         0│         1│         1│         2│       155│        100.00│      7.17│     9757.89│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     80│      1000│         0│       482│            30│         0│         1│         1│         2│       168│        100.00│      8.33│     9601.20│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │     90│      1000│         0│       435│            34│         0│         1│         1│         2│       194│        100.00│      9.23│     9755.34│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │    100│      1000│         0│       492│            38│         0│         1│         1│         2│       203│        100.00│     10.32│     9690.74│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │    200│      1000│         0│      4326│            81│         0│         1│         1│         2│       420│         99.55│     20.67│     9630.51│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │    300│      1000│         0│      5002│           119│         1│         1│         1│        53│       449│         83.48│     28.17│     8890.59│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │    500│      1000│         0│      4985│           118│         0│         1│         1│        11│       521│         50.23│     28.01│     8966.82│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │    750│      1000│         0│      4735│           127│         0│         1│         1│         2│       585│         32.22│     27.38│     8825.87│
    // ├───────┼──────────┼──────────┼──────────┼──────────────┼──────────┼──────────┼──────────┼──────────┼──────────┼──────────────┼──────────┼────────────┤
    // │    800│      1000│         0│      3992│           101│         0│         1│         1│         2│       497│         26.25│     23.95│     8767.65│
    // └───────┴──────────┴──────────┴──────────┴──────────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────────┴──────────┴────────────┘

    // setup metrics recorder
    enable_simple_recorder().unwrap();

    // number of concurrent peers to test (zcashd hardcaps `max_peers` to 873 on my machine)
    let synth_counts = vec![
        1, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 200, 300, 500, 750, 800,
    ];

    let mut table = RequestsTable::default();

    // start node, with max peers set so that our peers should
    // never be rejected.
    let mut node = Node::new().unwrap();
    node.initial_action(Action::WaitForConnection)
        .max_peers(synth_counts.iter().max().unwrap() * 2 + 10)
        .start()
        .await
        .unwrap();
    let node_addr = node.addr();

    for synth_count in synth_counts {
        // clear metrics and register metrics
        recorder::clear();
        metrics::register_histogram!(METRIC_LATENCY);

        // create N peer nodes which send M ping's as fast as possible
        let mut synth_handles = Vec::with_capacity(synth_count);
        let test_start = tokio::time::Instant::now();
        for _ in 0..synth_count {
            synth_handles.push(tokio::spawn(simulate_peer(node_addr)));
        }

        // wait for peers to complete
        for handle in synth_handles {
            let _ = handle.await;
        }

        let time_taken_secs = test_start.elapsed().as_secs_f64();

        // get latency stats
        let latencies = recorder::histograms()
            .lock()
            .get(&metrics::Key::from_name(METRIC_LATENCY))
            .unwrap()
            .value
            .clone();

        // add stats to table display
        table.add_row(RequestStats::new(
            synth_count as u16,
            PINGS,
            latencies,
            time_taken_secs,
        ));
    }

    node.stop().unwrap();

    // Display results table
    println!("{}", table);
}

async fn simulate_peer(node_addr: SocketAddr) {
    // Create a synthetic node, enable handshaking and auto-reply
    let mut synth_node = SyntheticNode::builder()
        .with_full_handshake()
        .with_all_auto_reply()
        .build()
        .await
        .unwrap();
    synth_node.connect(node_addr).await.unwrap();

    for _ in 0..PINGS {
        let nonce = Nonce::default();
        let expected = Message::Pong(nonce);

        // send Ping(nonce)
        synth_node
            .send_direct_message(node_addr, Message::Ping(nonce))
            .unwrap();

        let now = tokio::time::Instant::now();
        match synth_node
            .recv_message_timeout(Duration::from_secs(5))
            .await
        {
            Ok((_, reply)) => {
                assert_eq!(reply, expected);
                metrics::histogram!(METRIC_LATENCY, duration_as_ms(now.elapsed()));
            }
            Err(_timeout) => break,
        }
    }
}
