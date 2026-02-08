use anyhow::{Context, Result};
use ort::{
    execution_providers::{CUDAExecutionProvider, ExecutionProvider},
    session::Session,
    value::Tensor,
};

#[cfg(feature = "coreml")]
use ort::execution_providers::CoreMLExecutionProvider;

#[cfg(feature = "coreml")]
use ort::execution_providers::coreml::ComputeUnits as CoreMLComputeUnits;

use crate::infer::preprocess::PreprocessedInput;
use crate::infer::{ModelData, PoseSegBackend, RawOutput, SegRawOutput};

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
        pose: &ModelData,
        seg: &ModelData,
        input_size: u32,
        require_cuda: bool,
    ) -> Result<Self> {
        #[cfg(feature = "coreml")]
        let coreml = CoreMLExecutionProvider::default();
        #[cfg(feature = "coreml")]
        let coreml_available = coreml.is_available().unwrap_or(false);

        let cuda = CUDAExecutionProvider::default();
        let cuda_available = cuda
            .is_available()
            .context("failed to query CUDA execution provider availability")?;

        if require_cuda && !cuda_available {
            return Err(anyhow::anyhow!(
                "CUDA execution provider is not available in this ONNX Runtime build"
            ));
        }

        let (pose, ep_name_pose) = Self::build_session(
            pose,
            #[cfg(feature = "coreml")]
            &coreml,
            #[cfg(feature = "coreml")]
            coreml_available,
            &cuda,
            cuda_available,
            require_cuda,
        )?;
        let (seg, ep_name_seg) = Self::build_session(
            seg,
            #[cfg(feature = "coreml")]
            &coreml,
            #[cfg(feature = "coreml")]
            coreml_available,
            &cuda,
            cuda_available,
            require_cuda,
        )?;

        if let Some(name) = ep_name_pose.or(ep_name_seg) {
            println!("{} execution provider enabled.", name);
        }

        Ok(Self {
            pose,
            seg,
            input_size,
        })
    }

    fn build_session(
        model: &ModelData,
        #[cfg(feature = "coreml")] coreml: &CoreMLExecutionProvider,
        #[cfg(feature = "coreml")] coreml_available: bool,
        cuda: &CUDAExecutionProvider,
        cuda_available: bool,
        require_cuda: bool,
    ) -> Result<(OrtSession, Option<&'static str>)> {
        let mut builder = Session::builder()?;
        let mut ep_enabled: Option<&'static str> = None;

        #[cfg(feature = "coreml")]
        if coreml_available && !require_cuda {
            let coreml_configured =
                CoreMLExecutionProvider::default().with_compute_units(CoreMLComputeUnits::CPUOnly);
            match coreml_configured.register(&mut builder) {
                Ok(()) => {
                    ep_enabled = Some("CoreML");
                }
                Err(err) => {
                    eprintln!("Failed to register CoreML execution provider: {err}");
                    eprintln!("Falling back to other providers.");
                }
            }
        }

        if ep_enabled.is_none() && cuda_available {
            match cuda.register(&mut builder) {
                Ok(()) => {
                    ep_enabled = Some("CUDA");
                }
                Err(err) => {
                    if require_cuda {
                        return Err(err).context("failed to register CUDA execution provider");
                    }
                    eprintln!("Failed to register CUDA execution provider: {err}");
                    eprintln!("Falling back to CPU.");
                }
            }
        } else if ep_enabled.is_none() && !cuda_available && !require_cuda {
            eprintln!("No hardware acceleration available; using CPU.");
        }

        let session = builder
            .commit_from_memory(model.bytes())
            .with_context(|| format!("failed to open model ({})", model.label()))?;

        let input_name = session
            .inputs()
            .first()
            .context("missing model input")?
            .name()
            .to_string();
        let output_names = session
            .outputs()
            .iter()
            .map(|out| out.name().to_string())
            .collect::<Vec<_>>();

        Ok((
            OrtSession {
                session,
                input_name,
                output_names,
            },
            ep_enabled,
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
