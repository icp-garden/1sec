export const idlFactory = ({ IDL }) => {
	const Account = IDL.Record({
		owner: IDL.Principal,
		subaccount: IDL.Opt(IDL.Vec(IDL.Nat8))
	});
	const TaskType = IDL.Variant({
		NotifyOneSec: IDL.Record({
			scheduled_at: IDL.Nat64,
			ledger_canister_id: IDL.Text,
			receiver: Account
		}),
		ApproveMax: IDL.Record({
			target: IDL.Principal,
			ledger_canister_id: IDL.Principal
		}),
		RefreshExchangeRate: IDL.Null,
		UnwrapckUSD: IDL.Record({
			amount_e6s: IDL.Nat64,
			ledger_canister_id: IDL.Text,
			receiver: Account
		})
	});
	const Task = IDL.Record({ execute_at: IDL.Nat64, task_type: TaskType });
	const IcpAccount = IDL.Variant({ ICRC: Account, AccountId: IDL.Text });
	const UnwrapArgs = IDL.Record({
		from: IcpAccount,
		amount_e6s: IDL.Nat64,
		ledger_canister_id: IDL.Text
	});
	const Result = IDL.Variant({ Ok: IDL.Nat, Err: IDL.Text });
	return IDL.Service({
		compute_out_amount: IDL.Func([IDL.Nat64], [IDL.Nat64], ['query']),
		get_exchange_rate: IDL.Func([], [IDL.Opt(IDL.Nat)], ['query']),
		get_task_queue: IDL.Func([], [IDL.Vec(Task)], ['query']),
		unwrap_ck_to_onesec: IDL.Func([UnwrapArgs], [Result], [])
	});
};
export const init = ({ IDL }) => {
	return [];
};
