use pelite;
use pelite::pe64::*;
use pelite::pattern as pat;

pub fn print(bin: PeFile) {
	ignore_mouse_detection(bin);
}

fn ignore_mouse_detection(bin: PeFile) {
	// Find access near
	// mov dword ptr [ptr], 0xFF1300C8
	let mut save = [0; 4];
	if bin.scanner().finds_code(pat!("488B05$ {'} 4532C0 4C8BCA C702???? 488B88u4 0FB641u1"), &mut save) {
		println!("Ignore mouse detection:");
		println!("[[Overwatch.exe!{:#x}] + {:#x}] + {:#x}", save[1], save[2], save[3]);
		println!();
	}
	else {
		eprintln!("Unable to find `ignore mouse detection` signature.");
	}
}

fn view_matrix(bin: PeFile) {
	let mut save = [0; 4];
	if bin.scanner().finds_code(pat!(""), &mut save) {

	}
	else {
		eprintln!("Unable to find `view matrix` signatures.");
	}
}
