<script lang="ts">
	import SearchingNetworks from '$lib/images/gifs/SearchingNetworks.gif';

	import { goto } from '$app/navigation';
	import { onDestroy } from 'svelte';
	import Icons from '../../shared/Icons.svelte';
	import Layout from '../../shared/layout.svelte';
	import { check_ping_status, goBack } from '$lib/services';

	let apiInProgress: boolean = true;

	setTimeout(async () => {
		if (apiInProgress) {
			try {
				let data: any = await check_ping_status();
				apiInProgress = false;
				data.code == 'success' ? goto('/link-machine') : goto('/no-internet');
			} catch (error) {
				console.error('Error: checking ping : ', error);
				apiInProgress = false;
				goto('/setup-failed', { state: { error: error } });
			}
		}
	}, 2000);

	onDestroy(() => {
		apiInProgress = false;
	});
</script>

<Layout>
	<div class="flex flex-grow flex-col" style="height:-webkit-fill-available">
		<div class=" relative flex flex-grow flex-col justify-end items-center gap-2">
			<div class="flex justify-center text-2xl">
				<span> Checking connectivity... </span>
			</div>
			<div class="mt-2">
				<img alt="check internet" src={SearchingNetworks} />
			</div>
		</div>
	</div>

	<footer slot="footer" class="h-full w-full bg-[#05070A73] backdrop-blur-3xl backdrop-filter">
		<div
			class="border-silver-gray flex h-full w-full flex-row items-center justify-between border-t-2 px-4 py-3"
		>
			<button
				class="flex h-[60px] w-[60px] items-center justify-center rounded-lg p-1 text-[#FAFBFC]"
				on:click={goBack}
			>
				<Icons name="left_arrow" width="60" height="60" />
			</button>
		</div>
	</footer>
</Layout>
