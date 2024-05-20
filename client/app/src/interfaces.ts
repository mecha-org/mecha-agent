export enum NEXT_BUTTON_TYPE {
    SHOW_CHECK_INTERNET = 'SHOW_CHECK_INTERNET',
    REFRESH = 'REFRESH',
    SETTINGS = 'SETTINGS',
    SHOW_MACHINE_INFO = 'SHOW_MACHINE_INFO',
    EXIT = 'EXIT',
};

export interface MachineDataType {
    id: string,
    name?: string,
    icon?: string
}