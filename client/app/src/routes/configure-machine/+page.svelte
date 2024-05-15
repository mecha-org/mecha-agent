<script lang="ts">
	import SearchingMachine from '$lib/images/gifs/SearchingMachine.gif';
	import { goto } from '$app/navigation';
	import { machineInfo } from '$lib/stores';
	import Layout from '../../shared/layout.svelte';
	import { get_machine_id } from '$lib/services';
	import { isFileServingAllowed } from 'vite';
	
	const get_machine_id_data = async () => {
		try {
			const data = await get_machine_id();
			machineInfo.set({ id: data.machine_id });
			goto('/setup-success');
		} catch (error) {
			console.error('SHOW ERROR PAGE!!!!', error);
			goto("/setup-failed", { state: {error: "fetching Machine data isFileServingAllowed, Try again"} });
		}
	};
	setTimeout(get_machine_id_data, 3000);
</script>

<Layout>
	<div class="flex flex-col" style="height:-webkit-fill-available">
		<div class="relative flex flex-grow flex-col items-center justify-center gap-2">
			<div>
				<img class="" alt="searching info" src={SearchingMachine} />
			</div>
			<div class="flex justify-center text-lg">
				<span> Fetching Machine Information ... </span>
			</div>
		</div>
	</div>

	<footer slot="footer" class="h-full w-full bg-[#05070A73] backdrop-blur-3xl backdrop-filter">
	</footer>
	
</Layout>
