use std::collections::HashMap;
use std::cell::RefCell;
use super::{
    Object,
    Program, 
    VertexShader,
    TessEvaluationShader,
    TessControlShader,
    GeometryShader,
    FragmentShader,
    ComputeShader,
};
use gl::{self, types::*};
use math::{Mat4, Vec3, Vec4, Rgba, Rgb};

impl Program {
    pub fn link_status(&self) -> bool {
        self.program_iv(gl::LINK_STATUS) != 0
    }
    pub fn try_from_shaders(shaders: &[GLuint]) -> Result<Self, String> {
        unsafe {
            let program = gl::CreateProgram();
            assert_ne!(program, 0);

            let mut nb_attached = 0;
            for shader in shaders.iter().filter(|&s| *s != 0) {
                gl::AttachShader(program, *shader);
                nb_attached += 1;
            }
            assert_ne!(nb_attached, 0);

            gl::LinkProgram(program);

            for shader in shaders.iter().filter(|&s| *s != 0)  {
                gl::DetachShader(program, *shader);
            }

            let program = Program(program);

            if program.link_status() {
                Ok(program)
            } else {
                Err(program.info_log())
            }
        }
    }
    pub fn try_from_stages(
        vert: Option<&VertexShader>,
        tesc: Option<&TessControlShader>,
        tese: Option<&TessEvaluationShader>,
        geom: Option<&GeometryShader>,
        frag: Option<&FragmentShader>
    ) -> Result<Self, String> {
        let shaders = [
            vert.map(|s| s.gl_id()).unwrap_or(0), 
            tesc.map(|s| s.gl_id()).unwrap_or(0),
            tese.map(|s| s.gl_id()).unwrap_or(0),
            geom.map(|s| s.gl_id()).unwrap_or(0),
            frag.map(|s| s.gl_id()).unwrap_or(0),
        ];
        Self::try_from_shaders(&shaders)
    }
    pub fn try_from_compute(cs: &ComputeShader) -> Result<Self, String> {
        Self::try_from_shaders(&[cs.gl_id()])
    }
    pub fn try_from_vert_frag(vs: &VertexShader, fs: &FragmentShader) -> Result<Self, String> {
        Self::try_from_shaders(&[vs.gl_id(), fs.gl_id()])
    }
    pub fn info_log(&self) -> String {
        use ::std::ptr;
        unsafe {
            let mut len: GLint = 0;
            gl::GetProgramiv(self.gl_id(), gl::INFO_LOG_LENGTH, &mut len);
            let mut buf: Vec<u8> = Vec::with_capacity((len-1) as usize); // -1 to skip trailing null
            buf.set_len((len-1) as _);
            gl::GetProgramInfoLog(self.gl_id(), len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);
            String::from_utf8(buf).unwrap_or("<UTF-8 error>".to_owned())
        }
    }
    pub fn attrib_location(&self, name: &[u8]) -> Option<GLint> {
        assert_eq!(0, *name.last().unwrap());
        let i = unsafe {
            gl::GetAttribLocation(self.gl_id(), name.as_ptr() as *const GLchar)
        };
        match i {
            -1 => None,
            i @ _ => Some(i),
        }
    }
    pub fn uniform_location(&self, name: &[u8]) -> Option<GLint> {
        assert_eq!(0, *name.last().unwrap());
        let i = unsafe {
            gl::GetUniformLocation(self.gl_id(), name.as_ptr() as *const GLchar)
        };
        match i {
            -1 => None,
            i @ _ => Some(i),
        }
    }
    /*
    // WISH: Refactor this into a program Builer (do before linking)
    pub fn bind_attrib_location(&self, loc: GLuint, name: &[u8]) {
        assert_eq!(name[name.len()-1], 0);
        unsafe {
            gl::BindAttribLocation(self.gl_id(), loc, name.as_ptr() as *const GLchar);
        }
    }
    */
    pub fn program_iv(&self, param: GLenum) -> GLint {
        let mut i = 0;
        unsafe {
            gl::GetProgramiv(self.gl_id(), param, &mut i);
        }
        i
    }
    pub fn nb_active_attribs(&self) -> usize {
        self.program_iv(gl::ACTIVE_ATTRIBUTES) as _
    }
    pub fn nb_active_uniforms(&self) -> usize {
        self.program_iv(gl::ACTIVE_UNIFORMS) as _
    }
    pub fn active_attrib_unchecked(&self, index: usize) -> Option<GLSLActiveVar> {
        self.active_var(index, gl::GetActiveAttrib, gl::GetAttribLocation)
    }
    pub fn active_uniform_unchecked(&self, index: usize) -> Option<GLSLActiveVar> {
        self.active_var(index, gl::GetActiveUniform, gl::GetUniformLocation)
    }
    pub fn active_attrib(&self, index: usize) -> Option<GLSLActiveVar> {
        if index >= self.nb_active_attribs() {
            return None;
        }
        self.active_attrib_unchecked(index)
    }
    pub fn active_uniform(&self, index: usize) -> Option<GLSLActiveVar> {
        if index >= self.nb_active_uniforms() {
            return None;
        }
        self.active_uniform_unchecked(index)
    }
    // GL docs:
    //     If no information is available, length will be 0, and name will be an empty string.
    //     This situation could occur if this function is called after a link operation that failed.
    fn active_var(&self, i: usize, get_active_var: GLGetActiveVar, get_var_location: GLGetVarLocation) -> Option<GLSLActiveVar> {
        let mut name = [0_u8; 256];
        let mut name_len = 0;
        let mut array_len = 0;
        let mut var_type = 0;
        unsafe {
            (get_active_var)(self.gl_id(), i as _, name.len() as _, &mut name_len, &mut array_len, &mut var_type, name.as_mut_ptr() as _);
        }
        if name_len == 0 {
            return None;
        }
        let location = unsafe {
            (get_var_location)(self.gl_id(), name.as_ptr() as _)
        };
        assert_ne!(location, -1);
        Some(GLSLActiveVar {
            name: String::from_utf8(name[..name_len as usize].to_vec()).unwrap(),
            array_len,
            type_glenum: var_type,
            type_: GLSLType::try_from_glenum(var_type),
            location,
        })
    }
    pub fn active_attribs(&self) -> GLSLActiveVars {
        GLSLActiveVars::new(self, self.nb_active_attribs(), gl::GetActiveAttrib, gl::GetAttribLocation)
    }
    pub fn active_uniforms(&self) -> GLSLActiveVars {
        GLSLActiveVars::new(self, self.nb_active_uniforms(), gl::GetActiveUniform, gl::GetUniformLocation)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GLSLActiveVar {
    pub name: String,
    pub array_len: GLsizei,
    pub type_: Option<GLSLType>,
    pub type_glenum: GLenum,
    pub location: GLint,
}

type GLGetActiveVar = unsafe fn(GLuint, GLuint, GLsizei, *mut GLsizei, *mut GLint, *mut GLenum, *mut GLchar);
type GLGetVarLocation = unsafe fn(GLuint, *const GLchar) -> GLint;

pub struct GLSLActiveVars<'a> {
    prog: &'a Program,
    nb: usize,
    i: usize,
    get_active_var: GLGetActiveVar,
    get_var_location: GLGetVarLocation,
}

impl<'a> GLSLActiveVars<'a> {
    fn new(prog: &'a Program, nb: usize, get_active_var: GLGetActiveVar, get_var_location: GLGetVarLocation) -> Self {
        Self { prog, i: 0, nb, get_active_var, get_var_location }
    }
}

impl<'a> Iterator for GLSLActiveVars<'a> {
    type Item = GLSLActiveVar;
    fn next(&mut self) -> Option<GLSLActiveVar> {
        while self.i < self.nb {
            let item = self.prog.active_var(self.i, self.get_active_var, self.get_var_location);
            self.i += 1;
            if item.is_some() {
                return item;
            }
        }
        None
    }
}

macro_rules! gl_type_enum {
    ($($Type:ident = $GL:ident,)+) => {
        #[repr(u32)]
        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
        pub enum GLSLType {
            $($Type = gl::$GL,)+
        }
        impl GLSLType {
            pub fn try_from_glenum(e: GLenum) -> Option<Self> {
                match e {
                    $(gl::$GL => Some(GLSLType::$Type),)+
                    _ => None
                }
            }
        }
    }
}

gl_type_enum!{
    Float           = FLOAT            ,
    FloatVec2       = FLOAT_VEC2       ,
    FloatVec3       = FLOAT_VEC3       ,
    FloatVec4       = FLOAT_VEC4       ,
    FloatMat2       = FLOAT_MAT2       ,
    FloatMat3       = FLOAT_MAT3       ,
    FloatMat4       = FLOAT_MAT4       ,
    FloatMat2x3     = FLOAT_MAT2x3     ,
    FloatMat2x4     = FLOAT_MAT2x4     ,
    FloatMat3x2     = FLOAT_MAT3x2     ,
    FloatMat3x4     = FLOAT_MAT3x4     ,
    FloatMat4x2     = FLOAT_MAT4x2     ,
    FloatMat4x3     = FLOAT_MAT4x3     ,
    Int             = INT              ,
    IntVec2         = INT_VEC2         ,
    IntVec3         = INT_VEC3         ,
    IntVec4         = INT_VEC4         ,
    UnsignedInt     = UNSIGNED_INT     ,
    UnsignedIntVec2 = UNSIGNED_INT_VEC2,
    UnsignedIntVec3 = UNSIGNED_INT_VEC3,
    UnsignedIntVec4 = UNSIGNED_INT_VEC4,
    Double          = DOUBLE           ,
    DoubleVec2      = DOUBLE_VEC2      ,
    DoubleVec3      = DOUBLE_VEC3      ,
    DoubleVec4      = DOUBLE_VEC4      ,
    DoubleMat2      = DOUBLE_MAT2      ,
    DoubleMat3      = DOUBLE_MAT3      ,
    DoubleMat4      = DOUBLE_MAT4      ,
    DoubleMat2x3    = DOUBLE_MAT2x3    ,
    DoubleMat2x4    = DOUBLE_MAT2x4    ,
    DoubleMat3x2    = DOUBLE_MAT3x2    ,
    DoubleMat3x4    = DOUBLE_MAT3x4    ,
    DoubleMat4x2    = DOUBLE_MAT4x2    ,
    DoubleMat4x3    = DOUBLE_MAT4x3    ,

    Bool                                 = BOOL                                     , 
    BoolVec2                             = BOOL_VEC2                                ,
    BoolVec3                             = BOOL_VEC3                                ,
    BoolVec4                             = BOOL_VEC4                                ,
    Sampler1D                            = SAMPLER_1D                               ,
    Sampler2D                            = SAMPLER_2D                               ,
    Sampler3D                            = SAMPLER_3D                               ,
    SamplerCube                          = SAMPLER_CUBE                             ,
    Sampler1DShadow                      = SAMPLER_1D_SHADOW                        ,
    Sampler2DShadow                      = SAMPLER_2D_SHADOW                        ,
    Sampler1DArray                       = SAMPLER_1D_ARRAY                         ,
    Sampler2DArray                       = SAMPLER_2D_ARRAY                         ,
    Sampler1DArrayShadow                 = SAMPLER_1D_ARRAY_SHADOW                  ,
    Sampler2DArrayShadow                 = SAMPLER_2D_ARRAY_SHADOW                  ,
    Sampler2DMultisample                 = SAMPLER_2D_MULTISAMPLE                   ,
    Sampler2DMultisampleArray            = SAMPLER_2D_MULTISAMPLE_ARRAY             ,
    SamplerCubeShadow                    = SAMPLER_CUBE_SHADOW                      ,
    SamplerBuffer                        = SAMPLER_BUFFER                           ,
    Sampler2DRect                        = SAMPLER_2D_RECT                          ,
    Sampler2DRectShadow                  = SAMPLER_2D_RECT_SHADOW                   ,
    IntSampler1D                         = INT_SAMPLER_1D                           ,
    IntSampler2D                         = INT_SAMPLER_2D                           ,
    IntSampler3D                         = INT_SAMPLER_3D                           ,
    IntSamplerCube                       = INT_SAMPLER_CUBE                         ,
    IntSampler1DArray                    = INT_SAMPLER_1D_ARRAY                     ,
    IntSampler2DArray                    = INT_SAMPLER_2D_ARRAY                     ,
    IntSampler2DMultisample              = INT_SAMPLER_2D_MULTISAMPLE               ,
    IntSampler2DMultisampleArray         = INT_SAMPLER_2D_MULTISAMPLE_ARRAY         ,
    IntSamplerBuffer                     = INT_SAMPLER_BUFFER                       ,
    IntSampler2DRect                     = INT_SAMPLER_2D_RECT                      ,
    UnsignedIntSampler1D                 = UNSIGNED_INT_SAMPLER_1D                  ,
    UnsignedIntSampler2D                 = UNSIGNED_INT_SAMPLER_2D                  ,
    UnsignedIntSampler3D                 = UNSIGNED_INT_SAMPLER_3D                  ,
    UnsignedIntSamplerCube               = UNSIGNED_INT_SAMPLER_CUBE                ,
    UnsignedIntSampler1DArray            = UNSIGNED_INT_SAMPLER_1D_ARRAY            ,
    UnsignedIntSampler2DArray            = UNSIGNED_INT_SAMPLER_2D_ARRAY            ,
    UnsignedIntSampler2DMultisample      = UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE      ,
    UnsignedIntSampler2DMultisampleArray = UNSIGNED_INT_SAMPLER_2D_MULTISAMPLE_ARRAY,
    UnsignedIntSamplerBuffer             = UNSIGNED_INT_SAMPLER_BUFFER              ,
    UnsignedIntSampler2DRect             = UNSIGNED_INT_SAMPLER_2D_RECT             ,
    Image1D                              = IMAGE_1D                                 ,
    Image2D                              = IMAGE_2D                                 ,
    Image3D                              = IMAGE_3D                                 ,
    Image2DRect                          = IMAGE_2D_RECT                            ,
    ImageCube                            = IMAGE_CUBE                               ,
    ImageBuffer                          = IMAGE_BUFFER                             ,
    Image1DArray                         = IMAGE_1D_ARRAY                           ,
    Image2DArray                         = IMAGE_2D_ARRAY                           ,
    Image2DMultisample                   = IMAGE_2D_MULTISAMPLE                     ,
    Image2DMultisampleArray              = IMAGE_2D_MULTISAMPLE_ARRAY               ,
    IntImage1D                           = INT_IMAGE_1D                             ,
    IntImage2D                           = INT_IMAGE_2D                             ,
    IntImage3D                           = INT_IMAGE_3D                             ,
    IntImage2DRect                       = INT_IMAGE_2D_RECT                        ,
    IntImageCube                         = INT_IMAGE_CUBE                           ,
    IntImageBuffer                       = INT_IMAGE_BUFFER                         ,
    IntImage1DArray                      = INT_IMAGE_1D_ARRAY                       ,
    IntImage2DArray                      = INT_IMAGE_2D_ARRAY                       ,
    IntImage2DMultisample                = INT_IMAGE_2D_MULTISAMPLE                 ,
    IntImage2DMultisampleArray           = INT_IMAGE_2D_MULTISAMPLE_ARRAY           ,
    UnsignedIntImage1D                   = UNSIGNED_INT_IMAGE_1D                    ,
    UnsignedIntImage2D                   = UNSIGNED_INT_IMAGE_2D                    ,
    UnsignedIntImage3D                   = UNSIGNED_INT_IMAGE_3D                    ,
    UnsignedIntImage2DRect               = UNSIGNED_INT_IMAGE_2D_RECT               ,
    UnsignedIntImageCube                 = UNSIGNED_INT_IMAGE_CUBE                  ,
    UnsignedIntImageBuffer               = UNSIGNED_INT_IMAGE_BUFFER                ,
    UnsignedIntImage1DArray              = UNSIGNED_INT_IMAGE_1D_ARRAY              ,
    UnsignedIntImage2DArray              = UNSIGNED_INT_IMAGE_2D_ARRAY              ,
    UnsignedIntImage2DMultisample        = UNSIGNED_INT_IMAGE_2D_MULTISAMPLE        ,
    UnsignedIntImage2DMultisampleArray   = UNSIGNED_INT_IMAGE_2D_MULTISAMPLE_ARRAY  ,
    UnsignedIntAtomicCounter             = UNSIGNED_INT_ATOMIC_COUNTER              ,

    SamplerCubeMapArray                  = SAMPLER_CUBE_MAP_ARRAY                   ,
    SamplerCubeMapArrayShadow            = SAMPLER_CUBE_MAP_ARRAY_SHADOW            ,
    IntSamplerCubeMapArray               = INT_SAMPLER_CUBE_MAP_ARRAY               ,
    UnsignedIntSamplerCubeMapArray       = UNSIGNED_INT_SAMPLER_CUBE_MAP_ARRAY      ,
}



pub trait UniformElement: Sized {
    const GLSL_TYPE: GLSLType;
    fn gl_uniform(loc: GLint, m: &[Self]);
}

macro_rules! impl_gl_uniform_element {
    ($($T:ty: $GLSL:ident => $func:ident,)+) => {
        $(
            impl UniformElement for $T {
                const GLSL_TYPE: GLSLType = GLSLType::$GLSL;
                fn gl_uniform(loc: GLint, m: &[Self]) {
                    unsafe {
                        gl::$func(loc, m.len() as _, m.as_ptr() as _);
                    }
                }
            }
        )+
    }
}
impl UniformElement for Mat4<f32> {
    const GLSL_TYPE: GLSLType = GLSLType::FloatMat4;
    fn gl_uniform(loc: GLint, m: &[Self]) {
        unsafe {
            gl::UniformMatrix4fv(loc, m.len() as _, m[0].gl_should_transpose() as _, &m[0][(0, 0)]);
        }
    }
}

impl_gl_uniform_element!{
    Vec4<f32>: FloatVec4 => Uniform4fv,
    Vec3<f32>: FloatVec3 => Uniform3fv,
    Rgba<f32>: FloatVec4 => Uniform4fv,
    Rgb <f32>: FloatVec3 => Uniform3fv,
    u32: UnsignedInt => Uniform1uiv,
    i32: Int => Uniform1iv,
    f32: Float => Uniform1fv,
}


/// A ProgramEx caches uniform information in a HashMap to allow setting uniforms
/// in a fast and safe way.
#[derive(Debug, PartialEq, Eq)]
pub struct ProgramEx {
    program: Program,
    uniforms: HashMap<String, GLSLActiveVar>,
    // For more complex stuff such as "u_foobar[2].field[0]"
    extra_uniform_locations: RefCell<HashMap<String, GLint>>,
}

impl ProgramEx {
    pub fn new(program: Program) -> Self {
        let uniforms = program.active_uniforms().map(|v| (v.name.clone(), v)).collect();
        Self {
            program,
            uniforms,
            extra_uniform_locations: Default::default(),
        }
    }
    pub fn inner(&self) -> &Program {
        &self.program
    }
    pub fn into_inner(self) -> Program {
        self.program
    }
    pub fn uniform(&self, name: &str) -> Option<&GLSLActiveVar> {
        self.uniforms.get(name)
    }
    pub fn set_uniform_primitive<T: UniformElement>(&self, name: &str, value: &[T]) {
        self.set_uniform(name, T::GLSL_TYPE, value)
    }
    pub fn set_uniform<T: UniformElement>(&self, name: &str, ty: GLSLType, value: &[T]) {
        let mut extra_uniform_locations = self.extra_uniform_locations.borrow_mut();
        let location;
        if let Some(uniform) = self.uniform(name) {
            assert_eq!(uniform.array_len, value.len() as _);
            assert_eq!(uniform.type_, Some(ty));
            location = uniform.location;
        } else if let Some(loc) = extra_uniform_locations.get(name).map(Clone::clone) {
            location = loc;
        } else {
            let cstring = ::std::ffi::CString::new(name).unwrap();
            match self.program.uniform_location(cstring.as_bytes_with_nul()) {
                None => panic!("No such uniform: `{}`", name),
                Some(loc) => {
                    location = loc;
                    extra_uniform_locations.insert(cstring.into_string().unwrap(), location);
                },
            }
        }
        assert_ne!(location, -1);
        self.set_uniform_unchecked(location, value);
    }
    pub fn set_uniform_unchecked<T: UniformElement>(&self, location: GLint, value: &[T]) {
        T::gl_uniform(location, value);
    }
}

impl From<Program> for ProgramEx { fn from(p: Program) -> Self { Self::new(p) } }
impl From<ProgramEx> for Program { fn from(p: ProgramEx) -> Self { p.into_inner() } }
