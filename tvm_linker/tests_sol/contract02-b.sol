pragma solidity ^0.5.0;

contract MyContract {

	uint64 m_value;
	
	// A method to be called from another contract
	
	function remoteMethod(uint64 value) public {
		m_value = value;
		return; 
	}
	
}
