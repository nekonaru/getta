export interface Toast {
	id: number;
	message: string;
	kind: "info" | "error";
}

class ToastStore {
	items = $state<Toast[]>([]);
	private nextId = 1;

	push(message: string, kind: Toast["kind"] = "info"): void {
		const id = this.nextId++;
		this.items = [...this.items.slice(-2), { id, message, kind }];
		const ttl = kind === "error" ? 8000 : 4500;
		setTimeout(() => this.dismiss(id), ttl);
	}

	dismiss(id: number): void {
		this.items = this.items.filter((t) => t.id !== id);
	}
}

export const toastStore = new ToastStore();
