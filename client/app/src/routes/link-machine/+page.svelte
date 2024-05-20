<script lang="ts">
	import Header from '../../shared/Header.svelte';
	import { Progress } from '$lib/components/ui/progress';
	import { onDestroy, onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api';
	import { goto } from '$app/navigation';
	import Layout from '../../shared/layout.svelte';
	import Icons from '../../shared/Icons.svelte';
	import SubHeader from '../../shared/SubHeader.svelte';
	import { generate_code, provision_by_code } from '$lib/services';

	let provision_code: string = '';
	let is_code_generated: boolean = false;
	let is_code_provision: boolean = false;

	let error_message: string = '';
	let apiInProgress: boolean = true;
	let is_error: boolean = false;

	let timeout = 60;

	const generate_code_req = async () => {
		timeout = 60;
		if (!apiInProgress) apiInProgress = true;

		if (apiInProgress) {
			try {
				const data = await generate_code();
				provision_code = data?.code;
				is_code_generated = true;
			} catch (error: any) {
				console.error('Error: generate code : ', error);
				is_error = true;
				error_message = 'Something went wrong! Try again!'; // check
			}
		}
	};

	onMount(() => {
		generate_code_req();
	});

	const generate_code_process = setInterval(async () => {
		await generate_code_req();
	}, 60000);

	const provision_code_process = setInterval(async () => {
		if (is_code_generated) {
			// //
			try {
				const data = await provision_by_code(provision_code);
				console.log('provision_code_req data: ', data);
				is_code_provision = data?.success == true;
				apiInProgress = data?.success == true;
				if (!data.success) {
					apiInProgress = false;
					error_message = 'Incorrect code! Try again!';
				}
			} catch (error: any) {
				apiInProgress = false;
				console.error('Error: provision code : ', error);
				if (!error.toLowerCase().includes('parse response')) {
					is_error = true;
					error_message = 'Something went wrong! Try again!'; // check
				}
			}
			// //
		}
	}, 20000);

	const update_timer = setInterval(() => {
		if (is_code_generated) {
			if (timeout <= 0) timeout = 60;
			timeout -= 1;
		}
	}, 1000);

	const clearIntervalProcess = () => {
		apiInProgress = false;
		clearInterval(generate_code_process);
		clearInterval(provision_code_process);
		clearInterval(update_timer);
	};

	$: if (is_code_provision) {
		clearIntervalProcess();
		goto('/configure-machine');
	} else if (is_error) {
		console.error('Link Machine error_message: ', error_message);
		goto('/setup-failed', { state: { error: error_message } });
	}

	onDestroy(() => {
		clearIntervalProcess();
	});

	const goBack = () => {
		history.back();
	};
</script>

<Layout>
	<div class="flex flex-col">
		<Header title={'Link Your Machine'} />
		<SubHeader text = {"Use the below code to onnect this machine to your mech.so account"}/>

		<div class="mt-1.5 h-12 relative flex-grow flex-col rounded-xl border-2 border-solid border-zinc-600 " style="background-color: #15171D;">
			<div class="text-2xl leading-loose tracking-widest px-4">
				{provision_code}
			</div>
		</div>
		<Progress value={timeout} max={60} class="-mt-0.5 mb-1"/>

		<div class="mt-4 flex flex-col gap-4 text-sm font-medium">
			<div class="mb-2 flex flex-row gap-x-2">
				<div class="h-min rounded-md border px-1.5">1</div>
				<p>Create a new account on Mecha, if not signed up eardiver.</p>
			</div>
			<div class="mb-2 flex flex-row gap-x-2">
				<div class="h-min rounded-md border px-1.5">2</div>
				<p>Navigate to Machines &gt; Add Machine</p>
			</div>
			<div class="mb-2 flex flex-row gap-x-2">
				<div class="h-min rounded-md border px-1.5">3</div>
				<p>Enter the code shown above when asked</p>
			</div>
		</div>
	</div>

	<footer slot="footer" class="h-full w-full bg-[#05070A73] backdrop-blur-3xl backdrop-filter">
		<div class="flex h-full w-full flex-row items-center justify-between px-4 py-3">
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-xl bg-[#2A2A2C] p-2 text-[#FAFBFC]"
				on:click={goBack}
			>
				<Icons name="back_icon" width="32" height="32" />
			</button>
		</div>
	</footer>
</Layout>
