/** Central modal ownership so Escape and shortcuts only reach the topmost dialog. */

export type ModalName = "palette" | "onboarding" | "recovery";

class ModalStore {
	stack = $state<ModalName[]>([]);

	get active(): ModalName | null {
		return this.stack[this.stack.length - 1] ?? null;
	}

	push(name: ModalName): void {
		if (!this.stack.includes(name)) this.stack = [...this.stack, name];
	}

	pop(name: ModalName): void {
		if (this.stack.includes(name)) this.stack = this.stack.filter((item) => item !== name);
	}

	isTop(name: ModalName): boolean {
		return this.active === name;
	}

	has(name: ModalName): boolean {
		return this.stack.includes(name);
	}

	hasBlocking(): boolean {
		return this.stack.some((name) => name === "onboarding" || name === "recovery");
	}
}

export const modalStore = new ModalStore();
