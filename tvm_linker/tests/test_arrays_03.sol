pragma solidity ^0.5.0;

contract TestArray {

    function main(uint32 idx, uint256[] memory myarray, uint32 idy) public pure returns (uint256) {
        return myarray[idx + idy];
    }
    
}
