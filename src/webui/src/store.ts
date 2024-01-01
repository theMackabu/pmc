import { persistentMap } from '@nanostores/persistent';

export interface SettingsStore {
	base: string;
	token?: string;
}

export const $settings = persistentMap<SettingsStore>('settings:', {
	base: '/',
});
