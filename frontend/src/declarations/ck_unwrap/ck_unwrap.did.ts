import type { Principal } from '@icp-sdk/core/principal';
import type { ActorMethod } from '@icp-sdk/core/agent';
import type { IDL } from '@icp-sdk/core/candid';

export interface Account {
	owner: Principal;
	subaccount: [] | [Uint8Array | number[]];
}
export type IcpAccount = { ICRC: Account } | { AccountId: string };
export type Result = { Ok: bigint } | { Err: string };
export interface Task {
	execute_at: bigint;
	task_type: TaskType;
}
export type TaskType =
	| {
			NotifyOneSec: {
				scheduled_at: bigint;
				ledger_canister_id: string;
				receiver: Account;
			};
	  }
	| {
			ApproveMax: { target: Principal; ledger_canister_id: Principal };
	  }
	| { RefreshExchangeRate: null }
	| {
			UnwrapckUSD: {
				amount_e6s: bigint;
				ledger_canister_id: string;
				receiver: Account;
			};
	  };
export interface UnwrapArgs {
	from: IcpAccount;
	amount_e6s: bigint;
	ledger_canister_id: string;
}
export interface _SERVICE {
	compute_out_amount: ActorMethod<[bigint], bigint>;
	get_exchange_rate: ActorMethod<[], [] | [bigint]>;
	get_task_queue: ActorMethod<[], Array<Task>>;
	unwrap_ck_to_onesec: ActorMethod<[UnwrapArgs], Result>;
}
export declare const idlFactory: IDL.InterfaceFactory;
export declare const init: (args: { IDL: typeof IDL }) => IDL.Type[];
