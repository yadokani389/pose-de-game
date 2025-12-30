use anyhow::{Context, Result};
use ort::{
    execution_providers::{CUDAExecutionProvider, ExecutionProvider},
    session::Session,
    value::Tensor,
};

use crate::infer::preprocess::PreprocessedInput;
use crate::infer::{PoseSegBackend, RawOutput, SegRawOutput};

struct OrtSession {
    session: Session,
    input_name: String,
    output_names: Vec<String>,
}

pub(crate) struct OrtBackend {
    pose: OrtSession,
    seg: OrtSession,
    input_size: u32,
}

impl OrtBackend {
    pub(crate) fn load(
        pose_path: &str,
        seg_path: &str,
        input_size: u32,
        require_cuda: bool,
    ) -> Result<Self> {
        let cuda = CUDAExecutionProvider::default();
        let cuda_available = cuda
            .is_available()
            .context("failed to query CUDA execution provider availability")?;

        if require_cuda && !cuda_available {
            return Err(anyhow::anyhow!(
                "CUDA execution provider is not available in this ONNX Runtime build"
            ));
        }

        let (pose, cuda_enabled_pose) =
            Self::build_session(pose_path, &cuda, cuda_available, require_cuda)?;
        let (seg, cuda_enabled_seg) =
            Self::build_session(seg_path, &cuda, cuda_available, require_cuda)?;

        if cuda_enabled_pose || cuda_enabled_seg {
            println!("CUDA execution provider enabled.");
        }

        Ok(Self {
            pose,
            seg,
            input_size,
        })
    }

    fn build_session(
        path: &str,
        cuda: &CUDAExecutionProvider,
        cuda_available: bool,
        require_cuda: bool,
    ) -> Result<(OrtSession, bool)> {
        let mut builder = Session::builder()?;
        let mut cuda_enabled = false;
        if cuda_available {
            match cuda.register(&mut builder) {
                Ok(()) => {
                    cuda_enabled = true;
                }
                Err(err) => {
                    if require_cuda {
                        return Err(err).context("failed to register CUDA execution provider");
                    }
                    eprintln!("Failed to register CUDA execution provider: {err}");
                    eprintln!("Falling back to CPU.");
                }
            }
        } else {
            eprintln!("CUDA execution provider is not available; falling back to CPU.");
        }

        let session = builder
            .commit_from_file(path)
            .with_context(|| format!("failed to open model at {path}"))?;

        let input_name = session
            .inputs
            .first()
            .context("missing model input")?
            .name
            .to_string();
        let output_names = session
            .outputs
            .iter()
            .map(|out| out.name.to_string())
            .collect::<Vec<_>>();

        Ok((
            OrtSession {
                session,
                input_name,
                output_names,
            },
            cuda_enabled,
        ))
    }

    fn run_session(session: &mut OrtSession, input: &PreprocessedInput) -> Result<Vec<RawOutput>> {
        let input_tensor = Tensor::from_array((
            vec![
                1usize,
                3,
                input.input_size as usize,
                input.input_size as usize,
            ],
            input.data.clone(),
        ))?;

        let outputs = session.session.run(ort::inputs![
            session.input_name.as_str() => input_tensor
        ])?;

        let mut results = Vec::with_capacity(session.output_names.len());
        for name in &session.output_names {
            let output = outputs.get(name).context("missing model output")?;
            let (shape, data) = output.try_extract_tensor::<f32>()?;
            let dims: Vec<usize> = shape.iter().map(|d| (*d).max(0) as usize).collect();
            results.push(RawOutput {
                data: data.to_vec(),
                dims,
            });
        }

        Ok(results)
    }
}

impl PoseSegBackend for OrtBackend {
    fn input_size(&self) -> u32 {
        self.input_size
    }

    fn infer_pose(&mut self, input: &PreprocessedInput) -> Result<RawOutput> {
        let mut outputs = Self::run_session(&mut self.pose, input)?;
        outputs.pop().context("missing pose output from ORT")
    }

    fn infer_seg(&mut self, input: &PreprocessedInput) -> Result<SegRawOutput> {
        let outputs = Self::run_session(&mut self.seg, input)?;
        if outputs.len() < 2 {
            return Err(anyhow::anyhow!(
                "ORT seg outputs missing: expected 2, got {}",
                outputs.len()
            ));
        }
        Ok(SegRawOutput {
            dets: outputs[0].clone(),
            proto: outputs[1].clone(),
        })
    }
}
