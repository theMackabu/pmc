import { persistentMap } from '@nanostores/persistent';

export interface SettingsStore {
	token?: string;
	servers?: string;
}

export const $settings = persistentMap<SettingsStore>('settings:', {});
