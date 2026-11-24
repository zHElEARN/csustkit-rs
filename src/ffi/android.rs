use std::sync::atomic::{AtomicBool, Ordering};

use jni::{
    EnvUnowned,
    errors::ThrowRuntimeExAndDefault,
    jni_mangle,
    objects::{JClass, JObject},
};

use super::SessionInitError;

static TLS_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[jni_mangle("com.zhelearn.csustkit.CsustKitAndroid")]
pub fn initialize_tls<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
    context: JObject<'local>,
) {
    unowned_env
        .with_env(|env| -> jni::errors::Result<()> {
            rustls_platform_verifier::android::init_with_env(env, context)?;
            TLS_INITIALIZED.store(true, Ordering::Release);
            Ok(())
        })
        .resolve::<ThrowRuntimeExAndDefault>();
}

pub(super) fn check_initialized() -> Result<(), SessionInitError> {
    if TLS_INITIALIZED.load(Ordering::Acquire) {
        Ok(())
    } else {
        Err(SessionInitError::AndroidNotInitialized)
    }
}
