<script lang="ts">
	import Success from '$lib/images/gifs/Success.gif';
	import Layout from '../../shared/layout.svelte';
	import Icons from '../../Icons.svelte';
	import { goto } from '$app/navigation';

	const goNext = () => {
		goto('/machine-info');
	};

	const goBack = () => {
		history.back();
	};
</script>

<Layout>
	<div class="flex flex-grow flex-col" style="height:-webkit-fill-available">
		<div class=" relative flex flex-grow flex-col items-center justify-center gap-2">
			<div>
				<img alt="setup success" class="h-[200px] w-[200px]" src={Success} />
			</div>
			<div class="flex flex-col items-center justify-center text-base">
				Machine is now connected to your Mecha account
			</div>
		</div>
	</div>

	<footer slot="footer" class="h-full w-full bg-[#05070A73] backdrop-blur-3xl backdrop-filter">
		<div class="flex h-full w-full flex-row items-center justify-between px-4 py-3">
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-xl bg-[#15171D] p-2 text-[#FAFBFC]"
				on:click={goBack}
			>
				<Icons name="back_icon" width="32" height="32" />
			</button>
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-xl bg-[#15171D] p-2 text-[#FAFBFC]"
				on:click={goNext}
			>
				<Icons name="next_icon" width="32" height="32" />
			</button>
		</div>
	</footer>
</Layout>
