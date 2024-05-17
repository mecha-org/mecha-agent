import { writable } from 'svelte/store';
import type { MachineDataType } from '../interfaces';

export let machineInfo = writable<MachineDataType>({} as MachineDataType);

export const count = writable(0);