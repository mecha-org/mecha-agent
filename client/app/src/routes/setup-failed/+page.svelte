<script lang="ts">
	import Failure from '$lib/images/gifs/Failure.gif';
	import { page } from '$app/stores';
	import Layout from '../../shared/layout.svelte';
	import Icons from '../../shared/Icons.svelte';
	import { goBack } from '$lib/services';

	const state: any = $page.state;
	const error: any = state.error;
</script>

<Layout>
	<div class="flex flex-grow flex-col" style="height:-webkit-fill-available">
		<div class=" relative flex flex-grow flex-col items-center justify-center gap-2">
			<div class="">
				<img alt="setup failed" class="h-[230px] w-[230px]" src={Failure} />
			</div>
			<div class="flex flex-col items-center justify-center">
				<div class="text-2xl">Error connecting to service</div>
				<div
					class="mt-2 border-y-2 border-[#848484] bg-[#1C1C1C] px-2 py-1 text-base font-bold capitalize text-red-500"
				>
					Error : {error}
				</div>
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
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-lg p-2 text-[#FAFBFC]"
				on:click={goBack}
			>
				<Icons name="refresh_icon" width="60" height="60" />
			</button>
		</div>
	</footer>
</Layout>
