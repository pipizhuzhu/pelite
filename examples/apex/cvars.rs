use pelite;
use pelite::pe64::*;
use pelite::{util::CStr, Pod};
use pelite::pattern as pat;

pub fn print(bin: PeFile, dll_name: &str) {
	let cvars = convars(bin);
	let cmds = concommands(bin);

	println!("## ConVars\n");
	for cvar in &cvars {
		println!("<details>");
		println!("<summary><code>{}</code></summary>\n", cvar.name);
		if let Some(desc) = cvar.desc {
			println!("{}\n", desc);
		}
		println!("default: `{:?}`  ", cvar.default);
		println!("flags: `{:#x}`  ", cvar.flags);
		if let Some(min_value) = cvar.min_value {
			println!("min value: `{}`  ", min_value);
		}
		if let Some(max_value) = cvar.max_value {
			println!("max value: `{}`  ", max_value);
		}
		println!("</details>");
	}
	println!("\n### Addresses\n\n```");
	for cvar in &cvars {
		println!("{}!{:#010x} ConVar {}", dll_name, cvar.address, cvar.name);
	}
	println!("```\n");

	println!("## ConCommands\n");
	for cmd in &cmds {
		println!("<details>");
		println!("<summary><code>{}</code></summary>\n", cmd.name);
		if let Some(desc) = cmd.desc {
			println!("{}\n", desc);
		}
		println!("flags: `{:#x}`  ", cmd.flags);
		println!("</details>");
	}
	println!("\n### Addresses\n\n```");
	for cmd in &cmds {
		println!("{}!{:#010x} ConCommand {}", dll_name, cmd.address, cmd.name);
	}
	println!("```\n");
}

// Find information in the 'setinfo' command
// References "Custom user info value"
// sizeof(ConVar) == 160
#[allow(non_snake_case)]
#[derive(Pod, Debug)]
#[repr(C)]
pub struct RawConVar {
	// ConCommandBase
	pub vtable: u64,
	pub pNext: Ptr<RawConVar>,
	pub bRegistered: u8,
	pub pszName: Ptr<CStr>,
	pub pszHelpString: Ptr<CStr>,
	pub pszDataType: Ptr<CStr>,
	unk_u64: u64,
	pub fFlags: u32,
	// ConVar
	pub IConVar_vtable: u64,
	pub pParent: Ptr<RawConVar>,
	pub pszDefaultValue: Ptr<CStr>,
	pub pszString: u64, // Allocated
	pub StringLength: u64, // Length of allocated string
	pub fValue: f32,
	pub nValue: i32,
	pub bHasMin: u8,
	pub fMinVal: f32,
	pub bHasMax: u8,
	pub fMaxVal: f32,
	// Callback stuff...
	// callback_stuff: [u64; 4],
}

pub struct ConVar<'a> {
	pub address: u32,
	pub name: &'a str,
	pub desc: Option<&'a str>,
	pub data_type: &'a str,
	pub default: &'a str,
	pub flags: u32,
	pub min_value: Option<f32>,
	pub max_value: Option<f32>,
}

pub fn convars(bin: PeFile<'_>) -> Vec<ConVar<'_>> {
	// Find the main ConVar vtable
	let mut save = [0; 4];
	if !bin.scanner().finds_code(pat!("488BC8 488BD3 E8$ 4053 4883EC60 488BD9 C6411000 33C9 488D05$'"), &mut save) {
		eprintln!("ERR: unable to find ConVar vftable");
		return Vec::new();
	}
	// Get the virtual address of the ConVar vtable
	let vftable = bin.optional_header().ImageBase + save[1] as u64;
	// Find the data section
	let data_section = bin.section_headers().iter().find(|section| &section.Name == b".data\0\0\0").unwrap();
	// Scan the data section for pointers to the vtable
	let data_data = bin.derva_slice::<u64>(data_section.VirtualAddress, data_section.SizeOfRawData as usize / 8).unwrap();
	let mut convars = Vec::new();
	for i in data_data.iter().enumerate().filter_map(|(index, &ptr)| if ptr == vftable { Some(index) } else { None }) {
		let address = data_section.VirtualAddress + (i * 8) as u32;
		let raw = bin.derva::<RawConVar>(address).unwrap();
		let name = bin.deref_c_str(raw.pszName).unwrap_or(CStr::empty()).to_str().unwrap();
		let desc = bin.deref_c_str(raw.pszHelpString).ok().map(|desc| desc.to_str().unwrap());
		let data_type = bin.deref_c_str(raw.pszDataType).unwrap_or(CStr::empty()).to_str().unwrap();
		let default = bin.deref_c_str(raw.pszDefaultValue).unwrap_or(CStr::empty()).to_str().unwrap();
		let flags = raw.fFlags;
		let min_value = if raw.bHasMin != 0 { Some(raw.fMinVal) } else { None };
		let max_value = if raw.bHasMax != 0 { Some(raw.fMaxVal) } else { None };
		convars.push(ConVar { address, name, desc, data_type, default, flags, min_value, max_value });
	}

	// Sort to make a nice diff
	convars.sort_by_key(|convar| convar.name);
	convars
}

#[allow(non_snake_case)]
#[derive(Pod, Debug)]
#[repr(C)]
pub struct RawConCommand {
	// ConCommandBase
	pub vtable: u64,
	pub pNext: Ptr<RawConVar>,
	pub bRegistered: u8,
	pub pszName: Ptr<CStr>,
	pub pszHelpString: Ptr<CStr>,
	pub pszDataType: Ptr<CStr>, // Some string indicating the data type and min/max range in string form
	unk_u64: u64,
	pub fFlags: u32,
	// ConCommand
	unk_fn: u64,
	unk_zero: u64,
	pub fnCommandCallback: u64,
	pub fnCompletionCallback: u64,
	pub fnCommandType: u32,
}

pub struct ConCommand<'a> {
	pub address: u32,
	pub name: &'a str,
	pub desc: Option<&'a str>,
	pub flags: u32,
	pub callback: u32,
}

pub fn concommands(bin: PeFile<'_>) -> Vec<ConCommand<'_>> {
	// Find ConCommand constructor thingy
	let mut save = [0; 4];
	let pat = pat!("488D05${} 488D0D${'} 488905${'} E9$ 4053 4883EC20");
	let mut matches = bin.scanner().matches_code(pat);
	let mut concommands = Vec::new();
	while matches.next(&mut save) {
		if save[1] != save[2] {
			continue;
		}
		let address = save[1];
		let raw = bin.derva::<RawConCommand>(address).unwrap();
		let name = bin.deref_c_str(raw.pszName).unwrap_or(CStr::empty()).to_str().unwrap();
		let desc = bin.deref_c_str(raw.pszHelpString).ok().map(|desc| desc.to_str().unwrap());
		let flags = raw.fFlags;
		let callback = bin.va_to_rva(raw.fnCommandCallback).unwrap_or(0);
		concommands.push(ConCommand { address, name, desc, flags, callback })
	}
	concommands.sort_by_key(|concommand| concommand.name);
	concommands
}
