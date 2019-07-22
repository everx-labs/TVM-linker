pragma solidity ^0.5.0;

contract IRemoteContract {
	function acceptMoneyAndNumber(uint64 number) payable public;
}

contract IRemoteContractCallback {
	function sendMoneyAndNumberCallback(uint64 number) public;
}

contract MyContract is IRemoteContractCallback {

	uint64 m_result;

	function sendMoneyAndNumber(address remote, uint64 number) public {
		IRemoteContract(remote).acceptMoneyAndNumber.value(100000)(number);
		return;
	}
	
	// interface IRemoteContractCallback
	
	function sendMoneyAndNumberCallback(uint64 number) public {
		m_result = number;
		return;
	}
}
