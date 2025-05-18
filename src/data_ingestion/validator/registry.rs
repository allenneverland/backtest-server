use std::collections::HashMap;
use std::any::{Any, TypeId};
use std::sync::Arc;
use tokio::sync::RwLock;
use super::{
    traits::DataValidator,
    ohlcv_validator::OHLCVValidator,
    tick_validator::TickValidator,
};
use crate::domain_types::{OHLCVPoint, TickPoint};

/// Global validator registry
pub struct ValidationRegistry {
    // Store concrete instances, assuming types are known.
    ohlcv_validator: Arc<OHLCVValidator>,
    tick_validator: Arc<TickValidator>,
    // If you need to support more types, add them here or use a HashMap as originally designed.
    // For this refactor, sticking to the explicitly known types for simplicity.
    custom_validators: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl ValidationRegistry {
    pub fn new() -> Self {
        Self {
            ohlcv_validator: Arc::new(OHLCVValidator::new()),
            tick_validator: Arc::new(TickValidator::new()),
            custom_validators: HashMap::new(),
        }
    }

    // Register custom validator for a type T that implements DataValidator<T>
    pub fn register_custom_validator<T: 'static, V: DataValidator<T> + Send + Sync + 'static>(&mut self, validator: V) {
        self.custom_validators.insert(TypeId::of::<T>(), Arc::new(validator));
    }

    // Get a validator for a specific type T
    pub fn get_validator<T: 'static>(&self) -> Option<Arc<dyn DataValidator<T> + Send + Sync>> {
        if TypeId::of::<T>() == TypeId::of::<OHLCVPoint>() {
            // Downcasting Arc<OHLCVValidator> to Arc<dyn DataValidator<OHLCVPoint>>
            // This requires OHLCVValidator to implement DataValidator<OHLCVPoint>
            let validator = self.ohlcv_validator.clone() as Arc<dyn DataValidator<OHLCVPoint> + Send + Sync>;
            return Some(unsafe { std::mem::transmute(validator) }); // Unsafe transmute for type coercion
        }
        if TypeId::of::<T>() == TypeId::of::<TickPoint>() {
            let validator = self.tick_validator.clone() as Arc<dyn DataValidator<TickPoint> + Send + Sync>;
            return Some(unsafe { std::mem::transmute(validator) });
        }
        // 對於自定義驗證器，使用 unsafe 轉換
        self.custom_validators.get(&TypeId::of::<T>()).map(|v| {
            let v_clone = v.clone();
            // 使用 unsafe 轉換，與內置類型相同
            unsafe { std::mem::transmute(v_clone) }
        })
    }
}

// Global validation registry instance, wrapped in Arc<RwLock<...>> for thread-safe access
lazy_static::lazy_static! {
    static ref VALIDATION_REGISTRY: Arc<RwLock<ValidationRegistry>> = 
        Arc::new(RwLock::new(ValidationRegistry::new()));
}

// Get a clone of the Arc-wrapped global registry
pub fn get_registry_arc() -> Arc<RwLock<ValidationRegistry>> {
    VALIDATION_REGISTRY.clone()
}

// Asynchronously get a validator for type T. The validator V must be Clone.
// This now returns an Arc to the validator trait object.
pub async fn get_validator_for_type<T: 'static>() -> Option<Arc<dyn DataValidator<T> + Send + Sync>> {
    let registry_lock = VALIDATION_REGISTRY.read().await;
    registry_lock.get_validator::<T>()
}

// Example: Register a custom validator (async version)
pub async fn register_custom_validator_async<T: 'static, V: DataValidator<T> + Send + Sync + 'static>(validator: V) {
    let mut registry_lock = VALIDATION_REGISTRY.write().await;
    registry_lock.register_custom_validator::<T, V>(validator);
} 