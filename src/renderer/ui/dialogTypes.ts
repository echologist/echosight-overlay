export type DialogTone = 'info' | 'danger' | 'success';

export interface DialogOptions {
  cancelLabel?: string;
  confirmLabel?: string;
  title?: string;
  tone?: DialogTone;
}

export type AlertHandler = (message: string, options?: DialogOptions) => void | Promise<void>;
export type ConfirmHandler = (message: string, options?: DialogOptions) => boolean | Promise<boolean>;

export const ignoreAlert: AlertHandler = () => undefined;
export const denyConfirm: ConfirmHandler = () => false;
