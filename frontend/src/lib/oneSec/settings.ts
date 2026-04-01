import { icpAnonymous } from '$lib/user/icpUser';
import { Asset, type AssetKey } from './types';
import { MS_PER_SECOND, sleep } from './utils';
import * as fromCandid from './fromCandid';
import { TOKEN } from './config';

const UPDATE_INTERVAL_MS: number = 30 * MS_PER_SECOND;
const ERROR_COOLDOWN_MS: number = 10 * MS_PER_SECOND;

export interface BridgeSettings {
	src: Asset;
	dst: Asset;
	minAmount: number;
	maxAmount: number;
	available?: number;
	protocolFeeInPercent: number;
	transferFee: number;
	averageTransferFee: number;
	lastUpdated: Date;
}

export class BridgeSettingsStore {
	// Map: srcAsset -> dstAsset -> BridgeFee
	settings: Map<AssetKey, Map<AssetKey, BridgeSettings>> = new Map();
	pendingFetch?: Promise<void>;
	lastError?: Date;

	async get(src: Asset, dst: Asset): Promise<BridgeSettings> {
		const now = new Date();
		const srcKey = src.key();
		const dstKey = dst.key();
		const s = this.settings.get(srcKey)?.get(dstKey);
		if (s && now.getTime() - s.lastUpdated.getTime() <= UPDATE_INTERVAL_MS) {
			return s;
		}
		if (this.lastError) {
			const sinceLastErrorMs = now.getTime() - this.lastError.getTime();
			if (sinceLastErrorMs < ERROR_COOLDOWN_MS) {
				await sleep(ERROR_COOLDOWN_MS - sinceLastErrorMs);
			}
		}
		if (!this.pendingFetch) {
			this.pendingFetch = this.fetch();
		}
		try {
			await this.pendingFetch;
		} catch (err) {
			this.lastError = new Date();
			throw err;
		} finally {
			this.pendingFetch = undefined;
		}
		return this.settings.get(srcKey)!.get(dstKey)!;
	}

	update() {
		this.settings = new Map();
	}

	private async fetch() {
		const oneSec = icpAnonymous().oneSec();
		const fees = await oneSec.get_transfer_fees();
		const now = new Date();
		for (let fee of fees) {
			const sourceChain = fee.source_chain[0];
			const sourceToken = fee.source_token[0];
			const destinationChain = fee.destination_chain[0];
			const destinationToken = fee.destination_token[0];
			if (
				sourceChain === undefined ||
				sourceToken === undefined ||
				destinationChain === undefined ||
				destinationToken === undefined
			) {
				continue;
			}
			const src = fromCandid.asset(sourceChain, sourceToken);
			const srcKey = src.key();
			const dst = fromCandid.asset(destinationChain, destinationToken);
			const dstKey = dst.key();
			const decimals = TOKEN.get(src.token)?.decimals;
			if (decimals === undefined) {
				throw new Error(`no config for token ${src.token}`);
			}
			const value: BridgeSettings = {
				src,
				dst,
				minAmount: fromCandid.amount(fee.min_amount, decimals),
				maxAmount: fromCandid.amount(fee.max_amount, decimals),
				available:
					fee.available[0] === undefined
						? undefined
						: fromCandid.amount(fee.available[0], decimals),
				protocolFeeInPercent: fee.protocol_fee_in_percent,
				transferFee: fromCandid.amount(fee.latest_transfer_fee_in_tokens, decimals),
				averageTransferFee: fromCandid.amount(fee.average_transfer_fee_in_tokens, decimals),
				lastUpdated: now
			};
			if (!this.settings.has(srcKey)) {
				this.settings.set(srcKey, new Map());
			}
			this.settings.get(srcKey)?.set(dstKey, value);
		}
	}
}
