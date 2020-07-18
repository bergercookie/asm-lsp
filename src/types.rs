use std::collections::HashMap;
use strum;
use strum_macros::EnumString;

// Instruction ------------------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct Instruction {
    pub name: String,
    pub summary: String,
    pub forms: Vec<InstructionForm>,
    pub url: Option<String>,
    // TODO - Add example?
}

impl Default for Instruction {
    fn default() -> Self {
        let name = String::new();
        let summary = String::new();
        let forms = vec![];
        let url = None;

        Self {
            name,
            summary,
            forms,
            url,
        }
    }
}

impl Instruction {
    /// Add a new form at the current instruction
    pub fn push_form(&mut self, form: InstructionForm) {
        self.forms.push(form);
    }
}

// InstructionForm --------------------------------------------------------------------------------
#[derive(Default, Debug, Clone)]
pub struct InstructionForm {
    pub gas_name: Option<String>,
    pub go_name: Option<String>,
    pub mmx_mode: Option<MMXMode>,
    pub xmm_mode: Option<XMMMode>,
    pub cancelling_inputs: Option<bool>,
    pub nacl_version: Option<u8>,
    pub nacl_zero_extends_outputs: Option<bool>,
    pub isa: Option<ISA>,
    pub operands: Vec<Operand>,
}

impl InstructionForm {}

// helper structs ---------------------------------------------------------------------------------
pub type InstructionSet = HashMap<String, Instruction>;

#[derive(Debug, Clone, EnumString)]
pub enum XMMMode {
    SSE,
    AVX,
}

#[derive(Debug, Clone, EnumString)]
pub enum MMXMode {
    FPU,
    MMX,
}

// Instruction Set Architecture -------------------------------------------------------------------
#[derive(Debug, Clone, EnumString)]
pub enum ISA {
    RDTSC,
    RDTSCP,
    CPUID,
    CMOV,
    MMX,
    #[strum(serialize="MMX+")]
    MMXPlus,
    FEMMS,
    #[strum(serialize="3dnow!")]
    _3DNow,
    #[strum(serialize="3dnow!+")]
    _3DNowPlus,
    SSE,
    SSE2,
    SSE3,
    SSSE3,
    #[strum(serialize="SSE4.1")]
    SSE4_1,//
    #[strum(serialize="SSE4.2")]
    SSE4_2, //
    SSE4A,
    AVX,
    AVX2,
    XOP,
    FMA3,
    FMA4,
    F16C,
    PCLMULQDQ,
    AES,
    SHA,
    RDRAND,
    RDSEED,
    MOVBE,
    POPCNT,
    LZCNT,
    BMI,
    BMI2,
    TBM,
    ADX,
    CLFLUSH,
    CLFLUSHOPT,
    CLWB,
    CLZERO,
    PREFETCH,
    PREFETCHW,
    PREFETCHWT1,
    MONITOR,
    MONITORX,
    AVX512F,
    AVX512BW,
    AVX512DQ,
    AVX512VL,
    AVX512PF,
    AVX512ER,
    AVX512CD,
    AVX512VBMI,
    AVX512IFMA,
    AVX512VPOPCNTDQ,
}

// Operand ----------------------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct Operand {
    pub type_: OperandType,
    pub input: Option<bool>,
    pub output: Option<bool>,
    pub extended_size: Option<usize>
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, EnumString)]
pub enum OperandType {
    #[strum(serialize="1")]
    _1,
    #[strum(serialize="3")]
    _3,
    imm4,
    imm8,
    imm16,
    imm32,
    imm64,
    al,
    cl,
    r8,
    r8l,
    ax,
    r16,
    r16l,
    eax,
    r32,
    r32l,
    rax,
    r64,
    mm,
    xmm0,
    xmm,
    #[strum(serialize="xmm{k}")]
    xmm_k,
    #[strum(serialize="xmm{k_z}")]
    xmm_k_z,
    ymm,
    #[strum(serialize="ymm{k}")]
    ymm_k,
    #[strum(serialize="ymm{k_z}")]
    ymm_k_z,
    zmm,
    #[strum(serialize="zmm{k}")]
    zmm_k,
    #[strum(serialize="zmm{k_z}")]
    zmm_k_z,
    k,
    #[strum(serialize="k{k}")]
    k_k,
    moffs32,
    moffs64,
    m,
    m8,
    m16,
    #[strum(serialize="m16{k_z}")]
    m16_k_z,
    m32,
    #[strum(serialize="m32{k}")]
    m32_k,
    #[strum(serialize="m32{k_z}")]
    m32_k_z,
    m64,
    #[strum(serialize="m64{k}")]
    m64_k,
    #[strum(serialize="m64{k_z}")]
    m64_k_z,
    m128,
    #[strum(serialize="m128{k_z}")]
    m128_k_z,
    m256,
    #[strum(serialize="m256{k_z}")]
    m256_k_z,
    m512,
    #[strum(serialize="m512{k_z}")]
    m512_k_z,
    #[strum(serialize="m64/m32bcst")]
    m64_m32bcst,
    #[strum(serialize="m128/m32bcst")]
    m128_m32bcst,
    #[strum(serialize="m256/m32bcst")]
    m256_m32bcst,
    #[strum(serialize="m512/m32bcst")]
    m512_m32bcst,
    #[strum(serialize="m128/m64bcst")]
    m128_m64bcst,
    #[strum(serialize="m256/m64bcst")]
    m256_m64bcst,
    #[strum(serialize="m512/m64bcst")]
    m512_m64bcst,
    vm32x,
    #[strum(serialize="vm32x{k}")]
    vm32x_k,
    vm64x,
    #[strum(serialize="vm64x{k}")]
    vm64xk,
    vm32y,
    #[strum(serialize="vm32y{k}")]
    vm32yk_,
    vm64y,
    #[strum(serialize="vm64y{k}")]
    vm64y_k,
    vm32z,
    #[strum(serialize="vm32z{k}")]
    vm32z_k,
    vm64z,
    #[strum(serialize="vm64z{k}")]
    vm64z_k,
    rel8,
    rel32,
    #[strum(serialize="{er}")]
    er,
    #[strum(serialize="{sae}")]
    sae,
}
