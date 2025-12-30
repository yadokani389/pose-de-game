use anyhow::{Context, Result};
use openvino::{CompiledModel, Core, DeviceType, ElementType, InferRequest, Shape, Tensor};

use crate::infer::preprocess::PreprocessedInput;
use crate::infer::{PoseSegBackend, RawOutput, SegRawOutput};

struct OpenVinoSession {
    _compiled: CompiledModel,
    infer_request: InferRequest,
    input_tensor: Tensor,
    input_name: String,
    output_names: Vec<String>,
}

pub(crate) struct OpenVinoBackend {
    _core: Core,
    pose: OpenVinoSession,
    seg: OpenVinoSession,
    input_size: u32,
}

impl OpenVinoBackend {
    pub(crate) fn load(pose_path: &str, seg_path: &str, input_size: u32) -> Result<Self> {
        let model_pose_bytes = std::fs::read(pose_path)
            .with_context(|| format!("failed to read pose model at {pose_path}"))?;
        let model_seg_bytes = std::fs::read(seg_path)
            .with_context(|| format!("failed to read seg model at {seg_path}"))?;

        let mut core = Core::new()?;
        let pose = Self::build_session(&mut core, &model_pose_bytes, input_size, 1)?;
        let seg = Self::build_session(&mut core, &model_seg_bytes, input_size, 2)?;

        Ok(Self {
            _core: core,
            pose,
            seg,
            input_size,
        })
    }

    fn build_session(
        core: &mut Core,
        model_bytes: &[u8],
        input_size: u32,
        output_count: usize,
    ) -> Result<OpenVinoSession> {
        let model = core.read_model_from_buffer(model_bytes, None)?;
        let input_port = model.get_input_by_index(0)?;
        let input_name = input_port.get_name()?;

        let mut output_names = Vec::with_capacity(output_count);
        for i in 0..output_count {
            let output_port = model.get_output_by_index(i)?;
            output_names.push(output_port.get_name()?);
        }

        let mut compiled = core.compile_model(&model, DeviceType::CPU)?;
        let mut infer_request = compiled.create_infer_request()?;
        let input_shape = Shape::new(&[1, 3, input_size as i64, input_size as i64])?;
        let input_tensor = Tensor::new(ElementType::F32, &input_shape)?;
        infer_request.set_tensor(&input_name, &input_tensor)?;

        Ok(OpenVinoSession {
            _compiled: compiled,
            infer_request,
            input_tensor,
            input_name,
            output_names,
        })
    }

    fn run_session(
        session: &mut OpenVinoSession,
        input: &PreprocessedInput,
    ) -> Result<Vec<RawOutput>> {
        let buffer = session
            .input_tensor
            .get_data_mut::<f32>()
            .context("failed to map input tensor")?;
        if buffer.len() != input.data.len() {
            return Err(anyhow::anyhow!(
                "input tensor size mismatch: tensor={} data={}",
                buffer.len(),
                input.data.len()
            ));
        }
        buffer.copy_from_slice(&input.data);

        session
            .infer_request
            .set_tensor(&session.input_name, &session.input_tensor)?;
        session.infer_request.infer()?;

        let mut outputs = Vec::with_capacity(session.output_names.len());
        for name in &session.output_names {
            let output_tensor = session.infer_request.get_tensor(name)?;
            let shape = output_tensor.get_shape()?;
            let dims: Vec<usize> = shape
                .get_dimensions()
                .iter()
                .map(|d| (*d).max(0) as usize)
                .collect();
            let data = output_tensor.get_data::<f32>()?.to_vec();
            outputs.push(RawOutput { data, dims });
        }

        Ok(outputs)
    }
}

impl PoseSegBackend for OpenVinoBackend {
    fn input_size(&self) -> u32 {
        self.input_size
    }

    fn infer_pose(&mut self, input: &PreprocessedInput) -> Result<RawOutput> {
        let mut outputs = Self::run_session(&mut self.pose, input)?;
        outputs.pop().context("missing pose output from OpenVINO")
    }

    fn infer_seg(&mut self, input: &PreprocessedInput) -> Result<SegRawOutput> {
        let outputs = Self::run_session(&mut self.seg, input)?;
        if outputs.len() < 2 {
            return Err(anyhow::anyhow!(
                "OpenVINO seg outputs missing: expected 2, got {}",
                outputs.len()
            ));
        }
        Ok(SegRawOutput {
            dets: outputs[0].clone(),
            proto: outputs[1].clone(),
        })
    }
}
