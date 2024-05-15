import { invoke } from "@tauri-apps/api";

const check_ping_status = async() => {
    try {
        let response = await invoke('get_ping_status');
        return response;
    } catch (error) {
        throw error;
    }
};

const check_machine_provision_status = async () => {
    try {
        const data: any = await invoke('get_machine_provision_status');
        return data;
    } catch (error) {
        throw error;
    }
};

const get_machine_id = async () => {
    try {
        const data: any = await invoke('get_machine_id');
        return data;
    } catch (error) {
        throw error;
    }
};

const get_machine_info = async() => {
    try {
        let machine_name_data : any = await invoke('get_machine_info', {key: "identity.machine.name"});
        let machine_icon_data : any = await invoke('get_machine_info', {key: "identity.machine.icon_url"});
        return { name : machine_name_data.value, icon: machine_icon_data.value};
    } catch (error) {
        throw error;
    }
};

const generate_code = async() => {
    try {
        let data : any = await invoke('generate_code');
        return data;
    } catch (error) {
        throw error;
    }
};

const provision_by_code = async(code: string) => {
    try {
        let data : any = await invoke('provision_code', {code: code});
        return data;
    } catch (error) {
        throw error;
    }
};

export {
    check_ping_status,
    check_machine_provision_status,
    get_machine_id,
    get_machine_info,
    generate_code,
    provision_by_code
}