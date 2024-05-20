<script lang="ts">
	import MechaCompute from '$lib/images/svgs/MechaCompute.svg';
	import Layout from '../../shared/layout.svelte';
	import Icons from '../../shared/Icons.svelte';
	import { invoke } from '@tauri-apps/api';
	import { check_ping_status, get_machine_info } from '$lib/services';
	import { onMount } from 'svelte';
	import { machineInfo } from '$lib/stores';
	import type { MachineDataType } from '../../interfaces';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';

	let storeData: MachineDataType = $machineInfo;

	console.log('storeData: ', storeData);
	let machine_id: string = '-';
	let machine_name: string = 'My Machine';
	let machine_icon: string = '';
	let is_active: boolean = false;
	let is_error: boolean = false;
	let is_loading: boolean = true;

	let error_message: string = '';

	const get_machine_data = async () => {
		try {
			let data: any = await get_machine_info();
			let { id, name, icon } = data;
			machine_id = id;
			machine_name = name ? name : 'My Machine';
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
			is_error = true;
			error_message = 'Machine agent not running or there is no internet connectivity';
			console.error('Error: checking ping for status : ', error);
		}
	};

	onMount(() => {
		check_active_status();

		setTimeout(() => (is_loading = false), 3000);
		get_machine_data();
	});

	$: if (is_error) is_loading = false;

	setInterval(async () => {
		check_active_status();
	}, 10000);

	setInterval(() => {
		get_machine_data();
	}, 1200);

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
					width="120"
					height="120"
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
			<div>MACHINE ID</div>
			<div>{$machineInfo.id}</div>
		</div>

		<div class="mx-2 text-base font-medium">
			You can unlink your machine from your Mecha account
		</div>
		<Dialog.Root bind:open={is_error}>
			<Dialog.Content class="w-[90%] bg-[#15171D;] border-0 rounded-lg">
				<Dialog.Header>
					<Dialog.Title class="flex justify-center">
						<Icons name="attention" width="32" height="32" />
					</Dialog.Title>
					<Dialog.Description class="text-white">
						{error_message}
					</Dialog.Description>
				</Dialog.Header>
				<Dialog.Footer class="flex justify-center border-0">
					<Button
						class="bg-[#202431;] border-0 outline-none"
						type="button"
						on:click={() => {
							is_loading = false;
						}}>Okay</Button
					>
				</Dialog.Footer>
			</Dialog.Content>
		</Dialog.Root>

		<Dialog.Root bind:open={is_loading}>
			<Dialog.Content class="w-[90%] bg-[#15171D;] border-0 rounded-lg">
				<Dialog.Header>
					<Dialog.Title class="flex justify-center">
						<Icons name="info" width="32" height="32" />
					</Dialog.Title>
					<Dialog.Description class="text-white flex justify-center">
						Fetching machine information...
					</Dialog.Description>
				</Dialog.Header>
				<Dialog.Footer class="flex justify-center border-0">
					<Button
						class="bg-[#202431;] border-0 outline-none"
						type="button"
						on:click={() => {
							is_loading = false;
						}}>Okay</Button
					>
				</Dialog.Footer>
			</Dialog.Content>
		</Dialog.Root>
	</div>
	<footer slot="footer" class="h-full w-full bg-[#05070A73] backdrop-blur-3xl backdrop-filter">
		<div class="flex h-full w-full flex-row items-center justify-end px-4 py-3">
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-xl bg-[#2A2A2C] p-2 text-[#FAFBFC]"
				on:click={exitApp}
			>
				<Icons name="exit_icon" width="32" height="32" />
			</button>
		</div>
	</footer>
</Layout>
