use wasm_bindgen::prelude::*;

// #[global_allocator]
// static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub struct PeFile {
	data: Vec<u8>,
}

impl PeFile {
	fn pefile(&self) -> pelite::Result<pelite::PeFile<'_>> {
		pelite::PeFile::from_bytes(&self.data)
	}
}

#[wasm_bindgen]
impl PeFile {
	#[wasm_bindgen(constructor)]
	pub fn new(data: Vec<u8>) -> PeFile {
		PeFile { data }
	}
	pub fn base_relocs(&self) -> JsValue {
		JsValue::from_serde(&self.pefile().and_then(|pefile| pefile.base_relocs())).unwrap()
	}
	pub fn debug(&self) -> JsValue {
		JsValue::from_serde(&self.pefile().and_then(|pefile| pefile.debug())).unwrap()
	}
	// pub fn exception(&self) -> JsValue {
	// 	JsValue::from_serde(&self.pefile().and_then(|pefile| pefile.exception())).unwrap()
	// }
	pub fn exports(&self) -> JsValue {
		JsValue::from_serde(&self.pefile().and_then(|pefile| pefile.exports())).unwrap()
	}
	pub fn headers(&self) -> JsValue {
		JsValue::from_serde(&self.pefile().map(|pefile| pefile.headers())).unwrap()
	}
	pub fn imports(&self) -> JsValue {
		JsValue::from_serde(&self.pefile().and_then(|pefile| pefile.imports())).unwrap()
	}
	pub fn iat(&self) -> JsValue {
		JsValue::from_serde(&self.pefile().and_then(|pefile| pefile.iat())).unwrap()
	}
	pub fn load_config(&self) -> JsValue {
		JsValue::from_serde(&self.pefile().and_then(|pefile| pefile.load_config())).unwrap()
	}
	pub fn resources(&self) -> JsValue {
		JsValue::from_serde(&self.pefile().and_then(|pefile| pefile.resources())).unwrap()
	}
	pub fn rich_structure(&self) -> JsValue {
		JsValue::from_serde(&self.pefile().and_then(|pefile| pefile.rich_structure())).unwrap()
	}
	pub fn security(&self) -> JsValue {
		JsValue::from_serde(&self.pefile().and_then(|pefile| pefile.security())).unwrap()
	}
	pub fn tls(&self) -> JsValue {
		JsValue::from_serde(&self.pefile().and_then(|pefile| pefile.tls())).unwrap()
	}
}
