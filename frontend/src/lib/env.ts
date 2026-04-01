function getPublicVariable(key: string, fallback: string | boolean): string | boolean {
	// When code is bundled by Vite in the browser, import.meta.env will be defined.
	if (typeof import.meta !== 'undefined' && import.meta.env && import.meta.env[key] !== undefined) {
		return import.meta.env[key];
	}
	// In Node (e.g. during tests), fall back to process.env.
	const isNode =
		typeof process !== 'undefined' && process.versions != null && process.versions.node != null;

	if (isNode && process.env[key] !== undefined) {
		return process.env[key];
	}

	return fallback;
}

export const DFX_VERSION = getPublicVariable('VITE_DFX_VERSION', '0.24.3');
export const DFX_NETWORK = getPublicVariable('VITE_DFX_NETWORK', 'local');

export const CANISTER_ID_ONE_SEC_DAPP = getPublicVariable(
	'VITE_CANISTER_ID_ONE_SEC_DAPP',
	'5jlqy-lqaaa-aaaar-qbn6q-cai'
);

export const CANISTER_ID = getPublicVariable('VITE_CANISTER_ID', 'vy5lt-daaaa-aaaar-qblwa-cai');
export const STAGING = CANISTER_ID === 'vy5lt-daaaa-aaaar-qblwa-cai';
export const DEV = getPublicVariable('DEV', false) as boolean;

export const CANISTER_ID_ONE_SEC = STAGING
	? 'zvjow-lyaaa-aaaar-qap7q-cai'
	: '5okwm-giaaa-aaaar-qbn6a-cai';

export const CANISTER_ID_CK_UNWRAP = STAGING
	? '5ikyc-4yaaa-aaaar-qbzka-cai'
	: '5ikyc-4yaaa-aaaar-qbzka-cai';

export const CANISTER_ID_INTERNET_IDENTITY = getPublicVariable(
	'VITE_CANISTER_ID_INTERNET_IDENTITY',
	'rdmx6-jaaaa-aaaaa-aaadq-cai'
);
