pragma solidity ^0.5.0;

contract MyContract {

	uint256 m_value;
	
	// A method to be called from another contract
	
	function remoteMethod() public {
		m_value = address(this).balance;
		return; 
	}
	
}
