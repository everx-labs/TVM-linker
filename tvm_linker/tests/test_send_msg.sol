pragma solidity ^0.5.0;

contract RemoteContract {
	function on_get_allowance(uint64 amount) payable public;
}

contract MyContract {

	function get_allowance(address requester) payable public {
		// call remote contract
		RemoteContract r = RemoteContract(requester);
		r.on_get_allowance(0);
		r.on_get_allowance.value(1000)(0);
		return;
	}
	
}
