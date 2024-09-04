<script lang="ts">
	import SearchingNetworks from '$lib/images/gifs/SearchingNetworks.gif';
	import { goto } from '$app/navigation';
	import { machineInfo } from '$lib/stores';
	import Layout from '../../shared/layout.svelte';
	import { get_machine_id } from '$lib/services';

	const get_machine_id_data = new Promise(async (resolve, reject) => {
		await get_machine_id()
			.then((data: any) => {
				machineInfo.set({ id: data.machine_id });
				resolve(data);
			})
			.catch((error) => {
				console.error('SHOW ERROR PAGE!!!!', error);
				reject(error);
			});
	});

	const checkTimeout = new Promise((resolve, reject) => {
		setTimeout(reject, 15000, 'Timeout');
	});

	setTimeout(() => {
		Promise.race([get_machine_id_data, checkTimeout])
			.then((value) => {
				console.log('promise value: ', value);
				goto('/setup-success');
			})
			.catch((error) => {
				console.log('promise error: ', error);
				if (error == 'Timeout') {
					goto('/timeout-service');
				} else {
					goto('/setup-failed', {
						state: { error: 'Fetching machine data failed, Please try again' }
					});
				}
			});
	}, 3000);
</script>

<Layout>
	<div class="flex flex-col" style="height:-webkit-fill-available">
		<div class="relative flex flex-grow flex-col items-center justify-end">
			<div class="flex justify-center text-2xl">
				<span> Fetching machine information ... </span>
			</div>
			<div class="mt-2">
				<img alt="" src={SearchingNetworks} />
			</div>
		</div>
	</div>
</Layout>
