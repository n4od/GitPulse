import { create } from "zustand";

export type ToastTone = "info" | "success" | "error";

export interface Toast {
  id: string;
  title: string;
  description?: string;
  tone: ToastTone;
}

interface ToastState {
  toasts: Toast[];
  push: (toast: Omit<Toast, "id">) => string;
  dismiss: (id: string) => void;
}

/** Never show more than this at once — a stack of toasts is noise, not signal. */
const MAX_VISIBLE = 3;

export const useToastStore = create<ToastState>((set) => ({
  toasts: [],

  push: (toast) => {
    const id = crypto.randomUUID();
    set((state) => ({ toasts: [...state.toasts, { ...toast, id }].slice(-MAX_VISIBLE) }));
    return id;
  },

  dismiss: (id) => {
    set((state) => ({ toasts: state.toasts.filter((toast) => toast.id !== id) }));
  },
}));

export function toast(tone: ToastTone, title: string, description?: string): string {
  return useToastStore
    .getState()
    .push(description === undefined ? { tone, title } : { tone, title, description });
}
