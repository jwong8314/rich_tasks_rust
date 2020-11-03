10/28/20

- We walked through and commented rich_task_rust
  - Keys to note: Pin type pins the address of the object
  - BoxedFuture just means Pinned
  - Algo: while the ready_queue is not empty and not disconnected move stuff to
    the run_queue; otherwise run thing on queue until they're blocked; repeat
- Try to add spawn_with_priority to runtime/spawner.rs and task/spawn.rs
  - this just prints the priority
- Add test/udp_prio.rs which narrows down to just run one of the udp test
  requiring spawn
- run test: 
``` 
  cargo build --all-features
  cargo check --all-features
  cargo test udp_prio -- --nocapture 2>&1| grep -A 10 prio
```
- edited mod.rs and lib.rs to make the new fn visible 

11/03/20

- Abortable futures: https://docs.rs/futures/0.3.7/futures/future/struct.Abortable.html
Questions: 
- What do about GPU not being abortable?
- Can all scheduling decisions be reduced to priorities?
- How does the Timestamps work (what are coordinates and when are they set)?
- 
