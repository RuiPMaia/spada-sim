use pyo3::{prelude::*, types::PyModule};
use crate::gemm::{GEMM, CsrTuple};


pub fn load_pickled_gemms(gemm_fp: &str, gemm_nm: &str) -> PyResult<GEMM> {
    let code = r#"
def retrieve_pickled_csr(pickle_gemm_fp, pickle_gemm_name):
    print('--- Python Interface ---')
    import pickle
    import numpy as np
    from scipy.sparse import coo_matrix, csr_matrix, csc_matrix
    print(f'% Load {pickle_gemm_name} from', pickle_gemm_fp)
    with open(pickle_gemm_fp, 'rb') as f:
        gemms = pickle.load(f)

        A, B = gemms[pickle_gemm_name]
        if isinstance(A, (csc_matrix, coo_matrix)):
            A = A.tocsr()
        elif isinstance(A, (np.ndarray)):
            A = csr_matrix(A)
        else:
            raise TypeError('Unsupported matrix type: {}'.format(type(A)))

        if isinstance(B, (csc_matrix, coo_matrix)):
            B = B.tocsr()
        elif isinstance(B, (np.ndarray)):
            B = csr_matrix(B)
        else:
            raise TypeError('Unsupported matrix type: {}'.format(type(B)))

        shape_A, shape_B = A.shape, B.shape
        data_A, data_B = A.data, B.data
        indices_A, indices_B = A.indices, B.indices
        indptr_A, indptr_B = A.indptr, B.indptr

        print(f'% -- A --')
        print(f'% shape: {shape_A} data: {data_A[:5]}... indices: {indices_A[:5]}... indptr: {indptr_A[:5]}...')
        print(f'% shape: {shape_B} data: {data_B[:5]}... indices: {indices_B[:5]}... indptr: {indptr_B[:5]}...')
    print('--- Return from Python Interface ---\n')
    return (shape_A, indptr_A, indices_A, data_A, shape_B, indptr_B, indices_B, data_B)
    "#;

    let file_name = "retrieve_pickled_csr.py";
    let module_name = "retrieve_pickled_csr";

    Python::with_gil(|py| {
        let load_gemm_from_path = PyModule::from_code(
            py, code, file_name, module_name).unwrap();
        let csr_tuple: CsrTuple = 
            load_gemm_from_path.getattr("retrieve_pickled_csr").unwrap()
            .call1((gemm_fp, gemm_nm)).unwrap()
            .extract().unwrap();
        // println!("csr_tuple:\n {:#?}", csr_tuple);
        let gemm = GEMM::new(gemm_nm, csr_tuple);
        Ok(gemm)
    })

}