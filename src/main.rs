mod components;
mod frontend;
mod gemm;
mod pipeline_simu;
mod preprocessing;
mod py2rust;
mod storage;
mod storage_traffic_model;
mod util;
// mod oracle_storage_traffic_model;

use std::cmp::min;

use gemm::GEMM;
use storage::VectorStorage;
use storage_traffic_model::TrafficModel;

use crate::components::StreamBuffer;
use crate::frontend::{parse_config, Accelerator, Cli, Simulator, WorkloadCate};
use crate::pipeline_simu::PipelineSimulator;
use crate::preprocessing::affinity_based_row_reordering;
use crate::py2rust::load_pickled_gemms;
use crate::storage::CsrMatStorage;
use structopt::StructOpt;

// Workload included:
// ss: ['2cubes_sphere', 'amazon0312', 'ca-CondMat', 'cage12', 'cit-Patents',
// 'cop20k_A', 'email-Enron', 'filter3D', 'm133-b3', 'mario002', 'offshore', 'p2p-Gnutella31',
// 'patents_main', 'poisson3Da', 'roadNet-CA', 'scircuit', 'web-Google', 'webbase-1M', 'wiki-Vote',
// 'degme', 'EternityII_Etilde', 'Ge87H76', 'Ge99H100', 'gupta2', 'm_t1', 'Maragal_7', 'msc10848',
// 'nemsemm1', 'NotreDame_actors', 'opt1', 'raefsky3', 'ramage02', 'relat8', 'ship_001', 'sme3Db',
// 'vsp_bcsstk30_500sep_10in_1Kout', 'x104']
// nn: ['alexnetconv0', 'alexnetconv1', 'alexnetconv2', 'alexnetconv3', 'alexnetconv4',
// 'alexnetfc0', 'alexnetfc1', 'alexnetfc2', 'resnet50conv0', 'resnet50layer1_conv1',
// 'resnet50layer1_conv2', 'resnet50layer1_conv3', 'resnet50layer2_conv1', 'resnet50layer2_conv2',
// 'resnet50layer2_conv3', 'resnet50layer3_conv1', 'resnet50layer3_conv2', 'resnet50layer3_conv3',
// 'resnet50layer4_conv1', 'resnet50layer4_conv2', 'resnet50layer4_conv3', 'resnet50fc']

fn main() {
    // let omega_config = parse_config("omega_config_3mb.json").unwrap();
    let omega_config = parse_config("omega_config_1mb.json").unwrap();
    let cli: Cli = Cli::from_args();

    match cli.simulator {
        Simulator::TrafficModel => {
            let gemm_fp = match cli.category {
                WorkloadCate::NN => omega_config.nn_filepath,
                WorkloadCate::SS => omega_config.ss_filepath,
                WorkloadCate::Desired => omega_config.desired_filepath,
            };
            let gemm = load_pickled_gemms(&gemm_fp, &cli.workload).unwrap();
            let a_avg_row_len = gemm.a.nnz() / gemm.a.rows();
            let b_avg_row_len = gemm.b.nnz() / gemm.b.rows();
            println!("Get GEMM {}", gemm.name);
            println!("{}", &gemm);
            println!(
                "Avg row len of A: {}, Avg row len of B: {}",
                a_avg_row_len, b_avg_row_len
            );

            let validating_product_mat = (&gemm.a * &gemm.b).to_csr();

            let (mut dram_a, mut dram_b) = CsrMatStorage::init_with_gemm(gemm);
            let mut dram_psum = VectorStorage::new();

            // Preprocessing.
            if cli.preprocess {
                if let Some(rowmap) = affinity_based_row_reordering(
                    &mut dram_a,
                    omega_config.cache_size,
                    a_avg_row_len,
                    b_avg_row_len,
                ) {
                    dram_a.reorder_row(rowmap);
                }
            }

            let output_base_addr = dram_b.indptr.len();
            // Determine the default window & block shape.
            let default_reduction_window = match cli.accelerator {
                Accelerator::Ip | Accelerator::Omega => [omega_config.lane_num, 1],
                Accelerator::Op => [1, omega_config.lane_num],
            };

            let default_block_shape = match cli.accelerator {
                Accelerator::Ip => [omega_config.lane_num, 1],
                Accelerator::Omega => [omega_config.block_shape[0], omega_config.block_shape[1]],
                Accelerator::Op => [1, usize::MAX],
            };

            // Oracle execution: to use the optimal reduction window shape.
            let oracle_exec = true;

            let mut traffic_model = storage_traffic_model::TrafficModel::new(
                omega_config.pe_num,
                omega_config.lane_num,
                omega_config.cache_size,
                omega_config.word_byte,
                output_base_addr,
                default_reduction_window,
                default_block_shape,
                &mut dram_a,
                &mut dram_b,
                &mut dram_psum,
                cli.accelerator.clone(),
            );

            // let mut traffic_model = oracle_storage_traffic_model::TrafficModel::new(
            //         omega_config.pe_num,
            //         omega_config.lane_num,
            //         omega_config.cache_size,
            //         omega_config.word_byte,
            //         output_base_addr,
            //         default_reduction_window,
            //         default_block_shape,
            //         &mut dram_a,
            //         &mut dram_b,
            //         &mut dram_psum,
            //         cli.accelerator.clone(),
            //         oracle_exec,
            // );

            traffic_model.execute();
            
            let result = traffic_model.get_exec_result();
            let a_count = traffic_model.get_a_mat_stat();
            let b_count = traffic_model.get_b_mat_stat();
            let c_count = traffic_model.get_c_mat_stat();
            let exec_count = traffic_model.get_exec_round();
            let cache_count = traffic_model.get_cache_stat();

            println!("-----Result-----");
            println!("-----Access count");
            println!("Execution count: {}", exec_count);
            println!("A matrix count: read {} write {}", a_count.0, a_count.1);
            println!("B matrix count: read {} write {}", b_count.0, b_count.1);
            println!("C matrix count: read {} write {}", c_count.0, c_count.1);
            println!(
                "Cache count: read {} write {}",
                cache_count.0, cache_count.1
            );

            println!("-----Output product matrix");
            for idx in 0..min(result.len(), 10) {
                println!("{}", &result[idx]);
            }

            println!("----Validating output product matrix");
            let v_indptr = validating_product_mat.indptr().as_slice().unwrap().to_vec();
            let v_data = validating_product_mat.data().to_vec();
            let v_indices = validating_product_mat.indices().to_vec();

            for idx in 0..min(v_indptr.len() - 1, 10) {
                let sliced_len = min(v_indptr[idx + 1] - v_indptr[idx], 5);
                let sliced_indptr = &v_indices[v_indptr[idx]..v_indptr[idx] + sliced_len];
                let sliced_data = &v_data[v_indptr[idx]..v_indptr[idx] + sliced_len];
                println!(
                    "rowptr: {} indptr: {:?} data: {:?}",
                    &idx, sliced_indptr, sliced_data
                );
            }
        }

        Simulator::AccurateSimu => {
            // Cycle-accurate simulator.
            // TODO: Write the blocking mechanism.
            // TODO: Initialize the StreamBuffer component.
            // let mut omega = PipelineSimulator::new();
            // TODO: Add StreamBuffer to omega.
        }
    }
}

pub fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}
