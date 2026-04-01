import type { ApproveError } from '../declarations/icrc_ledger/icrc_ledger.did';
import type { TransferError, Icrc1TransferError } from '../declarations/icp_ledger/icp_ledger.did';
import { displayNumber } from '$lib/utils';

export const DEFAULT_ERROR_MESSAGE: string = 'Unknown result, please refresh the page.';

export function writeError(error: ApproveError | Icrc1TransferError | TransferError): string {
	switch (true) {
		case 'GenericError' in error:
			return `Generic Error (${error.GenericError.error_code}): ${error.GenericError.message}`;
		case 'TemporarilyUnavailable' in error:
			return 'Ledger is temporarily unavailable.';
		case 'AllowanceChanged' in error:
			return `Insufficient allowance: ${displayNumber({ value: error.AllowanceChanged.current_allowance, decimals: 8, decimalsToDisplay: 8 })}`;
		case 'Expired' in error:
			return `Approval expired: ${error.Expired.ledger_time}`;
		case 'BadBurn' in error:
			return `Bad burn: minimum burn amount is ${displayNumber({ value: error.BadBurn.min_burn_amount, decimals: 8, decimalsToDisplay: 8 })}`;
		case 'Duplicate' in error:
			return `Duplicate transaction of: ${error.Duplicate.duplicate_of}`;
		case 'BadFee' in error:
			if (typeof error.BadFee.expected_fee === 'bigint') {
				return `Bad fee, expected: ${displayNumber({ value: error.BadFee.expected_fee, decimals: 8, decimalsToDisplay: 8 })}`;
			} else {
				return `Bad fee, expected: ${displayNumber({ value: error.BadFee.expected_fee.e8s, decimals: 8, decimalsToDisplay: 8 })}`;
			}
		case 'CreatedInFuture' in error:
			return `Created in future: ${error.CreatedInFuture.ledger_time}`;
		case 'TooOld' in error:
			return 'The transaction is too old.';
		case 'InsufficientFunds' in error:
			if (typeof error.InsufficientFunds.balance === 'bigint') {
				return `Insufficient funds, balance: ${displayNumber({ value: error.InsufficientFunds.balance, decimals: 8, decimalsToDisplay: 8 })}`;
			} else {
				return `Insufficient funds, balance: ${displayNumber({ value: error.InsufficientFunds.balance.e8s, decimals: 8, decimalsToDisplay: 8 })}`;
			}
		case 'TxTooOld' in error:
			return 'The transaction is too old.';
		case 'TxDuplicate' in error:
			return `Duplicate transaction of: ${error.TxDuplicate.duplicate_of}`;
		case 'TxCreatedInFuture' in error:
			return 'Created in future.';
		default:
			return DEFAULT_ERROR_MESSAGE;
	}
}
