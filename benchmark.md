# Benchmark
Infos: Intel(R) Core(TM) i5-4460  CPU @ 3.20GHz, 4 cores, 23.4 GiB of memory

## Chunk generation (1 thread) ([Code](https://github.com/Lemonzyy/surface-nets-experiment/blob/e7e8bc2ac8f0d797de4320b76e2093feb37c8d0c/src/main.rs#L72-L103))

* ### Debug
`took 118.35651173s to generate 125000 chunks (946.852µs / chunk)`

* ### Release
`took 41.780775388s to generate 125000 chunks (334.246µs / chunk)`

## Entire chunk map generation & meshing (multiple threads)

* ### Using task pooling

    * #### Debug
    `took 11.9762255s to generate 8000 chunks (1.49703ms / chunk)`

    * #### Release
    `took 6.27468842s to generate 8000 chunks (784.336µs / chunk)`

* ### Using `concurrent-queue`

    * #### Debug
    `took 11.988958007s to generate 8000 chunks (1.49862ms / chunk)`

    * #### Release
    `took 6.103116472s to generate 8000 chunks (762.890µs / chunk)`

* ### Using `SegQueue` from `crossbeam`

    * #### Debug
    `took 11.9768596s to generate 8000 chunks (1.49711ms / chunk)`

    * #### Release
    `took 6.08528958s to generate 8000 chunks (760.661µs / chunk)`