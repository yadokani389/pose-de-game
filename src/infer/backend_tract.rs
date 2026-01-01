use anyhow::{Context, Result};
use tract_ndarray::Array4;
use tract_onnx::prelude::*;

use std::io::Cursor;

use crate::infer::preprocess::PreprocessedInput;
use crate::infer::{ModelData, PoseSegBackend, RawOutput, SegRawOutput};

pub(crate) struct TractBackend {
    pose_model: TypedRunnableModel<TypedModel>,
    seg_model: TypedRunnableModel<TypedModel>,
    input_size: u32,
}

impl TractBackend {
    pub(crate) fn load(pose: &ModelData, seg: &ModelData, input_size: u32) -> Result<Self> {
        let mut pose_reader = Cursor::new(pose.bytes());
        let pose_model = tract_onnx::onnx()
            .model_for_read(&mut pose_reader)
            .with_context(|| format!("failed to open pose model ({})", pose.label()))?
            .with_input_fact(
                0,
                f32::fact([1, 3, input_size as usize, input_size as usize]).into(),
            )?
            .into_optimized()?
            .into_runnable()?;

        let mut seg_reader = Cursor::new(seg.bytes());
        let seg_model = tract_onnx::onnx()
            .model_for_read(&mut seg_reader)
            .with_context(|| format!("failed to open seg model ({})", seg.label()))?
            .with_input_fact(
                0,
                f32::fact([1, 3, input_size as usize, input_size as usize]).into(),
            )?
            .into_optimized()?
            .into_runnable()?;

        Ok(Self {
            pose_model,
            seg_model,
            input_size,
        })
    }

    fn run_model(
        model: &TypedRunnableModel<TypedModel>,
        input: &PreprocessedInput,
    ) -> Result<RawOutput> {
        let tensor: Tensor = Array4::from_shape_vec(
            (1, 3, input.input_size as usize, input.input_size as usize),
            input.data.clone(),
        )
        .context("failed to build input tensor")?
        .into();

        let outputs = model.run(tvec![tensor.into()])?;
        let output = outputs
            .get(0)
            .context("missing model output")?
            .to_array_view::<f32>()?;
        let dims = output.shape().to_vec();
        let data = output.iter().copied().collect();

        Ok(RawOutput { data, dims })
    }
}

impl PoseSegBackend for TractBackend {
    fn input_size(&self) -> u32 {
        self.input_size
    }

    fn infer_pose(&mut self, input: &PreprocessedInput) -> Result<RawOutput> {
        Self::run_model(&self.pose_model, input)
    }

    fn infer_seg(&mut self, input: &PreprocessedInput) -> Result<SegRawOutput> {
        let tensor: Tensor = Array4::from_shape_vec(
            (1, 3, input.input_size as usize, input.input_size as usize),
            input.data.clone(),
        )
        .context("failed to build input tensor")?
        .into();

        let outputs = self.seg_model.run(tvec![tensor.into()])?;
        let dets = outputs
            .get(0)
            .context("missing seg det output")?
            .to_array_view::<f32>()?;
        let proto = outputs
            .get(1)
            .context("missing seg proto output")?
            .to_array_view::<f32>()?;

        let dets_dims = dets.shape().to_vec();
        let proto_dims = proto.shape().to_vec();
        let dets_data = dets.iter().copied().collect();
        let proto_data = proto.iter().copied().collect();

        Ok(SegRawOutput {
            dets: RawOutput {
                data: dets_data,
                dims: dets_dims,
            },
            proto: RawOutput {
                data: proto_data,
                dims: proto_dims,
            },
        })
    }
}
