export const TOAST_LIFETIME_MS = 5_000;

export class Toast {
	static next_id = 0;

	public id: number;
	public message: string;
	public type: 'success' | 'error' | 'warning';
	public isTemporary: boolean;
	public timeLeft = TOAST_LIFETIME_MS;

	constructor({
		message,
		type,
		isTemporary
	}: {
		message: string;
		type: 'success' | 'error' | 'warning';
		isTemporary: boolean;
	}) {
		this.id = ++Toast.next_id;
		this.message = message;
		this.type = type;
		this.isTemporary = isTemporary;
	}

	static temporarySuccess(message: string): Toast {
		return new Toast({
			message,
			type: 'success',
			isTemporary: true
		});
	}

	static temporaryError(message: string): Toast {
		return new Toast({
			message,
			type: 'error',
			isTemporary: true
		});
	}

	static temporaryWarning(message: string): Toast {
		return new Toast({
			message,
			type: 'warning',
			isTemporary: true
		});
	}

	static success(message: string): Toast {
		return new Toast({
			message,
			type: 'success',
			isTemporary: false
		});
	}

	static error(message: string): Toast {
		return new Toast({
			message,
			type: 'error',
			isTemporary: false
		});
	}

	static warning(message: string): Toast {
		return new Toast({
			message,
			type: 'warning',
			isTemporary: false
		});
	}

	decreaseTime(elapsed: number) {
		this.timeLeft -= elapsed;
	}
}
