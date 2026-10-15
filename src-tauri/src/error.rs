use serde::Serialize;

/// Error que cruza el puente IPC. Se serializa como un objeto con `kind` y
/// `message` para que el frontend pueda distinguir casos sin parsear texto.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("error de entrada/salida: {0}")]
    Io(#[from] std::io::Error),

    #[error("documento invalido: {0}")]
    InvalidDocument(String),

    #[error("no soportado: {0}")]
    Unsupported(String),

    #[error("error de serializacion: {0}")]
    Serde(#[from] serde_json::Error),
}

impl AppError {
    fn kind(&self) -> &'static str {
        match self {
            AppError::Io(_) => "io",
            AppError::InvalidDocument(_) => "invalidDocument",
            AppError::Unsupported(_) => "unsupported",
            AppError::Serde(_) => "invalidDocument",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
