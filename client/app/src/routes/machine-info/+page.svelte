<script lang="ts">
	import MechaCompute from '$lib/images/svgs/MechaCompute.svg';
	import Layout from '../../shared/layout.svelte';
	import Icons from '../../Icons.svelte';
	import { invoke } from '@tauri-apps/api';
	import { page } from '$app/stores';
	import { check_ping_status, get_machine_info } from '$lib/services';
	import { getContext, onMount } from 'svelte';
	import { machineInfo } from '$lib/stores';
	import type { MachineDataType } from '../../interfaces';
	import toast from 'svelte-french-toast';

	let storeData: MachineDataType = $machineInfo;

	let machine_id: string = '-';
	let machine_name: string = 'My Machine';
	let machine_icon: string = '';
	let is_active: boolean = false;

	const get_machine_data = async () => {
		try {
			let data: any = await get_machine_info();
			let { name, icon } = data;
			machine_name = name ? name : "My Machine";
			machine_icon = icon;
		} catch (error) {
			console.error('fetching machine info error : ', error);
		}
	};

	$: if (machine_name || machine_icon) {
		machineInfo.set({
			...storeData,
			name: machine_name,
			icon: machine_icon
		});
	}

	const check_active_status = async () => {
		try {
			let data: any = await check_ping_status();
			is_active = data?.code == 'success';
		} catch (error) {
			is_active = false;
			console.error('Error: checking ping for status : ', error);
			toast.error('Machine Agent not running or not internet connectivity');
		}
	};

	// let toast_text = String::from("Fetching Machine Info...");
	// let error_toast = String::from("Machine Agent not running or not internet connectivity");

	onMount(() => {
		check_active_status();

		toast.loading("Fetching Machine Info...", {duration: 2000});
		get_machine_data();
	});

	setInterval(async () => {
		check_active_status();
	}, 10000);

	setInterval(() => {
		get_machine_data();
	}, 7000);

	const exitApp = () => {
		invoke('exit_app');
	};
</script>

<Layout>
	<div class="flex h-full w-full flex-col justify-center" style="height:-webkit-fill-available">
		<div class="relative mx-2 flex flex-col items-center justify-center">
			<div class="">
				<img
					class="rounded-md"
					width="90"
					height="90"
					src={machine_icon == '' ? MechaCompute : machine_icon}
					alt="Machine Profile"
				/>
			</div>

			<div class="flex flex-row items-center gap-1">
				<p>{$machineInfo.name}</p>

				{#if is_active}
					<Icons name="active_status_icon" width="32" height="32" />
				{:else}
					<Icons name="idle_status_icon" width="32" height="32" />
				{/if}
			</div>
		</div>

		<div
			class="mx-2 my-2 flex flex-row justify-between rounded border border-solid border-zinc-600 p-4 text-base capitalize tracking-widest"
		>
			<div>Machine Id</div>
			<div>{$machineInfo.id}</div>
		</div>

		<div class="mx-2">You can unlink your machine from your Mecha account</div>
	</div>
	<footer slot="footer" class="h-full w-full bg-[#05070A73] backdrop-blur-3xl backdrop-filter">
		<div class="flex h-full w-full flex-row items-center justify-end px-4 py-3">
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-xl bg-[#15171D] p-2 text-[#FAFBFC]"
				on:click={exitApp}
			>
				<Icons name="exit_icon" width="32" height="32" />
			</button>
		</div>
	</footer>
</Layout>
