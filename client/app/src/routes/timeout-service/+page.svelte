<script lang="ts">
	import Layout from '../../shared/layout.svelte';
	import Icons from '../../shared/Icons.svelte';
	import timeout from '$lib/images/timeout.png';
	import { goBack } from '$lib/services';
</script>

<Layout>
	<div class="flex flex-grow flex-col" style="height:-webkit-fill-available">
		<div class=" relative flex flex-grow flex-col justify-center items-center gap-2 mt-8">
			<div class="mb-2.5">
				<img alt="timeout" src={timeout} />
			</div>
			<div class="flex justify-center text-2xl">
				<!-- <span> Request timed out, please try again </span> -->
				<span> Timeout! </span>
			</div>
		</div>
	</div>

	<footer slot="footer" class="h-full w-full bg-[#05070A73] backdrop-blur-3xl backdrop-filter">
		<div
			class="border-silver-gray flex h-full w-full flex-row items-center justify-end border-t-2 px-4 py-3"
		>
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-lg p-1 text-[#FAFBFC]"
				on:click={goBack}
			>
				<Icons name="refresh_icon" width="60" height="60" />
			</button>
		</div>
	</footer>
</Layout>
