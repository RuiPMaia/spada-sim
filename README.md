# Spada simulator
Fork that removes unnecessary prints.

## Install
Please first install the [Rust toolchain](https://www.rust-lang.org/tools/install).

The simulator interacts with [python3](https://www.python.org/downloads/) for parsing sparse matrices:
```bash
$ python3 -m venv spadaenv
$ source spadaenv/bin/activate
$ pip install -U pip numpy scipy
```

## Build
```bash
$ cargo build --release --no-default-features
```

## Workload
The simulator accepts both MatrixMarket (.mtx) and numpy formatted matrices, with the latter ones packed as a pickle file (.pkl). The folder containing these matrices is specified in the config file under `config`.

## Simulate
First ensure the created python virtual environment is activated. The following command simulates SpGEMM of [cari](https://sparse.tamu.edu/Meszaros/cari) on Spada with the configuration specified in `config/config_1mb_row1.json`.
```bash
(spadaenv) $ ./target/release/spada-sim accuratesimu spada ss cari config/config_1mb_row1.json
```
## Reference

If you use this tool in your research, please kindly cite the following paper.

Zhiyao Li, Jiaxiang Li, Taijie Chen, Dimin Niu, Hongzhong Zheng, Yuan Xie, and Mingyu Gao.
Spada: Accelerating Sparse Matrix Multiplication with Adaptive Dataflow.
In *Proceedings of the 28th International Conference on Architectural Support for Programming Languages and Operating Systems* (ASPLOS), 2023.
