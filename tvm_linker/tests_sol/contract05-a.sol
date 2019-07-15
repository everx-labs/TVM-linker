pragma solidity ^0.5.0;

contract IRemoteContract {
	function remoteMethod(uint16 x) public;
}

contract IRemoteContractCallback {
	function remoteMethodCallback(uint16 x) public;
}

contract MyContract is IRemoteContractCallback {

	uint m_result;

	function method(address anotherContract, uint16 x) public {
		IRemoteContract(anotherContract).remoteMethod(x);
		return;
	}
	
	// interface IRemoteContractCallback
	
	function remoteMethodCallback(uint16 x) public {
		m_result = x;
		return;
	}
}
