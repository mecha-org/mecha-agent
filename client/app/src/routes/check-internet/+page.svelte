<script lang="ts">
	import SearchingWifi from '$lib/images/gifs/SearchingWifi.gif';

	import { goto } from '$app/navigation';
	import { onDestroy } from 'svelte';
	import Icons from '../../shared/Icons.svelte';
	import Layout from '../../shared/layout.svelte';
	import { check_ping_status } from '$lib/services';

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

	const goBack = () => {
		history.back();
	};
</script>

<Layout>
	<div class="flex flex-grow flex-col" style="height:-webkit-fill-available">
		<div class=" relative flex flex-grow flex-col justify-center items-center gap-2">
			<div>
				<img alt="check internet" src={SearchingWifi} />
			</div>
			<div class="flex justify-center text-base">
				<span> Checking for internet connectivity ... </span>
			</div>
		</div>
	</div>

	<footer slot="footer" class="w-full h-full bg-[#05070A73] backdrop-filter backdrop-blur-3xl">
		<div class="flex w-full h-full flex-row items-center justify-between px-4 py-3">
			<button
				class="bg-[#2A2A2C] text-[#FAFBFC] p-2 rounded-xl w-[48px] h-[48px] flex items-center justify-center"
				on:click={goBack}
			>
				<Icons name="back_icon" width="32" height="32" />
			</button>
		</div>
	</footer>
</Layout>
